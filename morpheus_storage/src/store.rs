use failure::Fallible;
use log::*;
use std::cell::RefCell;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use crate::client::AttributeMap;
use crate::{messages, MsgPackRpc, ProfileId, ProfilePtr, ProfileStore, RpcProfile, RpcPtr};

pub struct DummyProfileStore {
    connect_timeout: Duration,
    addr: SocketAddr,
    rpc: RefCell<Option<RpcPtr<TcpStream, TcpStream>>>,
}

impl DummyProfileStore {
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
        if self.rpc.borrow().is_none() {
            debug!("Connecting to {:?}", self.addr);

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
}

impl ProfileStore for DummyProfileStore {
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
            profile.set_attribute_map(AttributeMap::default())?;
            Ok(Rc::new(RefCell::new(profile)) as ProfilePtr)
        })
    }

    /// There is no call specified for this method
    fn remove(&mut self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
