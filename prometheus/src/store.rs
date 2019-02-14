use failure::Fallible;
use std::cell::RefCell;
use std::net::{SocketAddr, TcpStream};
use std::rc::Rc;
use std::time::Duration;

use super::vault::*;
use morpheus_storage::*;

pub struct DummyProfileStore {
    connect_timeout: Duration,
    addr: SocketAddr,
    rpc: Option<RpcPtr<TcpStream, TcpStream>>,
}

impl DummyProfileStore {
    pub fn new(
        // vault: &mut DummyProfileVault,
        addr: &SocketAddr,
        connect_timeout: Duration,
    ) -> Fallible<Self> {
        // let id = if let Some(active_id) = vault.get_active()? {
        //     active_id
        // } else {
        //     vault.create_id()?
        // };
        Ok(Self {
            connect_timeout,
            addr: *addr,
            rpc: Option::None,
        })
    }

    fn rpc(&mut self) -> Fallible<RpcPtr<TcpStream, TcpStream>> {
        if let None = self.rpc {
            let tcp_stream = TcpStream::connect_timeout(&self.addr, self.connect_timeout)?;
            // TODO make timeouts configurable
            tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
            tcp_stream.set_write_timeout(Some(Duration::from_secs(5)))?;
            let tcp_stream_clone = tcp_stream.try_clone()?;
            let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);

            self.rpc = Option::Some(Rc::new(RefCell::new(rpc)));
        }

        Ok(self.rpc.clone().unwrap())
    }
}

impl ProfileStore for DummyProfileStore {
    fn get(&self, id: &ProfileId) -> Option<ProfilePtr> {
        Option::None
        //        Some()
    }
    fn create(&self, id: &ProfileId) -> Fallible<ProfilePtr> {
        unimplemented!()
    }
    fn remove(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
