use failure::Fallible;
use log::*;
use std::cell::RefCell;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use failure::err_msg;

use crate::client::AttributeMap;
use crate::{messages, MsgPackRpc, ProfileId, ProfilePtr, ProfileRepository, RpcProfile, RpcPtr};

pub struct RpcProfileRepository {
    connect_timeout: Duration,
    addr: SocketAddr,
    rpc: RefCell<Option<RpcPtr<TcpStream, TcpStream>>>,
}

impl RpcProfileRepository {
    pub fn new(addr: &SocketAddr, connect_timeout: Duration) -> Fallible<Self> {
        // let id = if let Some(active_id) = vault.get_active()? {
        //     active_id
        // } else {
        //     vault.create_id()?
        // };
        Ok(Self {
            connect_timeout,
            addr: *addr,
            rpc: RefCell::new(Option::None),
        })
    }

    fn rpc(&self) -> Fallible<RpcPtr<TcpStream, TcpStream>> {
        // TODO is really a lazy singleton init needed here? It makes types and
        //      everything much more complex, would be simpler in constructor
        if self.rpc.borrow().is_none() {
            debug!("Connecting to storage backend server {:?}", self.addr);

            let tcp_stream = TcpStream::connect_timeout(&self.addr, self.connect_timeout)?;
            // TODO make timeouts configurable
            tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            tcp_stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            let tcp_stream_clone = tcp_stream.try_clone()?;
            let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);

            *self.rpc.borrow_mut() = Option::Some(Rc::new(RefCell::new(rpc)));
        }

        Ok(self.rpc.borrow().clone().unwrap())
    }

    pub fn list_nodes(&self) -> Fallible<Vec<ProfileId>> {
        let params = messages::ListNodesParams {};
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
    fn get(&self, id: &ProfileId) -> Option<ProfilePtr> {
        self.rpc()
            .and_then(|rpc| {
                let rpc_clone = rpc.clone();
                Ok(Rc::new(RefCell::new(RpcProfile::new(id, rpc_clone))) as ProfilePtr)
            })
            .ok()
    }

    /// https://gitlab.libertaria.community/iop-stack/communication/morpheus-storage-daemon/wikis/Morpheus-storage-protocol#create-profile
    fn create(&mut self, id: &ProfileId) -> Fallible<ProfilePtr> {
        self.rpc().and_then(|rpc| {
            let request = messages::AddNodeParams { id: id.clone() };
            let rpc_clone = rpc.clone();
            let _res = rpc.borrow_mut().send_request("add_node", request)?;
            let profile = RpcProfile::new(id, rpc_clone);
            // TODO this shouldn't belong here, querying an empty attribute set shouldn't be an error
            profile.set_osg_attribute_map(AttributeMap::default())?;
            Ok(Rc::new(RefCell::new(profile)) as ProfilePtr)
        })
    }

    /// There is no call specified for this method
    fn remove(&mut self, _id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
