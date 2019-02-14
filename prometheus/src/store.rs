use failure::Fallible;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

use super::vault::*;
use morpheus_storage::*;

pub struct DummyProfileStore {
    profile: Arc<RwLock<Profile>>,
    connect_timeout: Duration,
    addr: SocketAddr,
    rpc: Option<Rc<RefCell<MsgPackRpc>>>,
}

impl DummyProfileStore {
    pub fn new(
        vault: &mut DummyProfileVault,
        addr: &SocketAddr,
        connect_timeout: Duration

    ) -> Fallible<Self> {
        let id = if let Some(active_id) = vault.get_active()? {
            active_id
        } else {
            vault.create_id()?
        };
        Ok(Self {
            connect_timeout,
            addr,
            rpc: Option::None
        })
    }

    fn rpc(&mut self) -> Fallible<MsgPackRpc> {
        if let None = self.rpc {
            let tcp_stream = TcpStream::connect_timeout(addr, timeout)?;
            // TODO make timeouts configurable
            tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            tcp_stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            let tcp_stream_clone = tcp_stream.try_clone()?;
            let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);

            self.rpc = Option::Some(rpc);

            
        } 

        self.rpc.unwrap().clone()
    }
    
}

impl ProfileStore for DummyProfileStore {
    fn get(&self, id: &ProfileId) -> Option<Profile> {
        Some(self.profile.clone())
    }
    fn create(&self, id: &ProfileId) -> Fallible<Profile> {
        unimplemented!()
    }
    fn remove(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
