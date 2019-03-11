use std::cell::RefCell;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use failure::{ensure, err_msg, Fallible};
use log::*;

use crate::client::{FallibleExtension, MsgPackRpc, RpcProfile, RpcPtr};
use crate::messages::{AddNodeParams, ListInEdgesParams, ListInEdgesReply, ListNodesParams};
use osg::model::{AttributeMap, Link, ProfileId};
use osg::profile::ProfilePtr;
use osg::repo::ProfileRepository;

pub struct RpcProfileRepository {
    address: SocketAddr,
    network_timeout: Duration,
    rpc: RefCell<Option<RpcPtr<TcpStream, TcpStream>>>,
}

impl RpcProfileRepository {
    pub fn new(address: &SocketAddr, network_timeout: Duration) -> Fallible<Self> {
        Ok(Self {
            address: *address,
            network_timeout,
            rpc: RefCell::new(Option::None),
        })
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
        let params = ListNodesParams {};
        let rpc = self.rpc()?;
        let response = rpc.borrow_mut().send_request("list_nodes", params)?;
        let node_vals = response
            .reply
            .ok_or_else(|| err_msg("Server returned no reply content for query"))?;
        let nodes = rmpv::ext::from_value(node_vals)?;
        Ok(nodes)
    }
}

impl ProfileRepository for RpcProfileRepository {
    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#show-profile
    fn get(&self, id: &ProfileId) -> Fallible<ProfilePtr> {
        self.rpc().and_then(|rpc| {
            let rpc_clone = rpc.clone();
            Ok(Rc::new(RefCell::new(RpcProfile::new(id, rpc_clone))) as ProfilePtr)
        })
    }

    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#create-profile
    fn set(&mut self, id: &ProfileId, profile: ProfilePtr) -> Fallible<()> {
        ensure!(
            *id != profile.borrow().id(),
            "Implementation error: RpcProfileRepository got conlicting key and value: {} vs {}",
            id,
            profile.borrow().id()
        );
        self.rpc().and_then(|rpc| {
            let request = AddNodeParams { id: id.clone() };
            let rpc_clone = rpc.clone();
            rpc.borrow_mut()
                .send_request("add_node", request)
                .map(|_r| ())
                .key_not_existed_or_else(|| Ok(()))?;

            let profile = RpcProfile::new(id, rpc_clone);
            // TODO this shouldn't belong here, querying an empty attribute set shouldn't be an error
            profile.set_osg_attribute_map(AttributeMap::default())?;
            Ok(Rc::new(RefCell::new(profile)) as ProfilePtr)
        })?;
        Ok(())
    }

    fn clear(&mut self, id: &ProfileId) -> Fallible<()> {
        let profile_ptr = self.get(id)?;
        let mut profile = profile_ptr.borrow_mut();

        let links = profile.links()?;
        for link in links {
            profile.remove_link(&link.peer_profile)?;
        }
        let attributes = profile.attributes()?;
        for attr in attributes.keys() {
            profile.clear_attribute(&attr)?;
        }
        Ok(())
    }

    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>> {
        self.rpc().and_then(|rpc| {
            let params = ListInEdgesParams { id: id.clone() };
            let response = rpc.borrow_mut().send_request("list_inedges", params)?;
            let reply_val = response
                .reply
                .ok_or_else(|| err_msg("Server returned no reply content for query"))?;
            let reply: ListInEdgesReply = rmpv::ext::from_value(reply_val)?;
            let followers = reply
                .into_iter()
                .map(|peer_profile| Link { peer_profile })
                .collect();
            Ok(followers)
        })
    }
}
