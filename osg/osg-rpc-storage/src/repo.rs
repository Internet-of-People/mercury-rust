use std::cell::RefCell;
use std::convert::TryFrom;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use failure::{err_msg, Fallible};
use log::*;

use crate::client::{FallibleExtension, MsgPackRpc, RpcProfile, RpcPtr};
use crate::messages;
use keyvault::PublicKey as KeyVaultPublicKey;
use osg::model::*;
use osg::profile::{Profile, ProfilePtr};
use osg::repo::{ProfileExplorer, ProfileRepository};

#[derive(Clone)]
pub struct RpcProfileRepository {
    address: SocketAddr,
    network_timeout: Duration,
    rpc: RefCell<Option<RpcPtr<TcpStream, TcpStream>>>,
}

impl RpcProfileRepository {
    pub fn new(address: &SocketAddr, network_timeout: Duration) -> Fallible<Self> {
        Ok(Self { address: *address, network_timeout, rpc: RefCell::new(Option::None) })
    }

    pub fn connect(
        address: &SocketAddr,
        network_timeout: Duration,
    ) -> Fallible<MsgPackRpc<TcpStream, TcpStream>> {
        debug!("Connecting to storage backend server {:?}", address);

        let tcp_stream = TcpStream::connect_timeout(&address, network_timeout)?;
        tcp_stream.set_read_timeout(Some(network_timeout))?;
        tcp_stream.set_write_timeout(Some(network_timeout))?;
        let tcp_stream_clone = tcp_stream.try_clone()?;
        let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);
        Ok(rpc)
    }

    fn rpc(&self) -> Fallible<RpcPtr<TcpStream, TcpStream>> {
        // TODO is really a lazy singleton init needed here? It makes types and
        //      everything much more complex, would be simpler in constructor
        if self.rpc.borrow().is_none() {
            let rpc = Self::connect(&self.address, self.network_timeout)?;
            *self.rpc.borrow_mut() = Option::Some(Rc::new(RefCell::new(rpc)));
        }

        Ok(self.rpc.borrow().clone().unwrap())
    }

    pub fn list_nodes(&self) -> Fallible<Vec<ProfileId>> {
        let params = messages::ListNodesParams {};
        let rpc = self.rpc()?;
        let response = rpc.borrow_mut().send_request("list_nodes", params)?;
        let node_vals =
            response.reply.ok_or_else(|| err_msg("Server returned no reply content for query"))?;
        let nodes = rmpv::ext::from_value(node_vals)?;
        Ok(nodes)
    }

    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#show-profile
    pub fn get_node(&self, id: &ProfileId) -> Fallible<ProfilePtr> {
        self.rpc().and_then(|rpc| {
            let rpc_profile = RpcProfile::new(id, rpc.clone());
            Ok(Rc::new(RefCell::new(rpc_profile)) as ProfilePtr)
        })
    }

    pub fn remove_node(&self, key: &PublicKey) -> Fallible<()> {
        self.rpc().and_then(|rpc| {
            let params = messages::RemoveNodeParams { id: key.key_id() };
            rpc.borrow_mut().send_request("remove_node", params)?;
            Ok(())
        })
    }
}

impl ProfileRepository for RpcProfileRepository {
    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#show-profile
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData> {
        let rpc_profile = self.get_node(id)?;
        ProfileData::try_from(rpc_profile)
    }

    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#create-profile
    fn set(&mut self, profile: ProfileData) -> Fallible<()> {
        match self.remove_node(&profile.public_key()) {
            Ok(()) => debug!("Profile existed, removed it as part of overwriting"),
            Err(_e) => debug!("Failed to remove profile, creating it as new one"),
        };

        self.rpc().and_then(|rpc| {
            let request = messages::AddNodeParams { id: profile.id() };
            rpc.borrow_mut()
                .send_request("add_node", request)
                .map(|_r| ())
                .key_not_existed_or_else(|| Ok(()))?;

            let mut rpc_profile = RpcProfile::new(&profile.id(), rpc);
            // TODO consider version conflict checks here
            rpc_profile.set_version(profile.version())?;
            rpc_profile.set_public_key(&profile.public_key())?;
            rpc_profile.set_osg_attribute_map(profile.attributes())?;

            for link in profile.links() {
                rpc_profile.create_link(&link.peer_profile)?;
            }
            Ok(())
        })?;
        Ok(())
    }

    fn clear(&mut self, key: &PublicKey) -> Fallible<()> {
        let profile = self.get(&key.key_id())?;
        self.set(ProfileData::tombstone(key, profile.version()))
    }
}

impl ProfileExplorer for RpcProfileRepository {
    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>> {
        self.rpc().and_then(|rpc| {
            let params = messages::ListInEdgesParams { id: id.clone() };
            let response = rpc.borrow_mut().send_request("list_inedges", params)?;
            let reply_val = response
                .reply
                .ok_or_else(|| err_msg("Server returned no reply content for query"))?;
            let reply: messages::ListInEdgesReply = rmpv::ext::from_value(reply_val)?;
            let followers = reply.into_iter().map(|peer_profile| Link { peer_profile }).collect();
            Ok(followers)
        })
    }
}
