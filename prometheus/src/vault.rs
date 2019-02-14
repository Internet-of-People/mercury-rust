//use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use failure::{bail, Fallible};

//use morpheus_keyvault::*;
use morpheus_storage::*;
//use crate::types::{Link, PublicKey, Signature};

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>;
    fn create_id(&self) -> Fallible<ProfileId>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&self, id: &ProfileId) -> Fallible<()>;
}

// TODO remove this dummy implementation completely and use the RpcProfileStore instead
pub struct DummyProfileVault {
    pub profile_id: ProfileId,
}

pub struct DummyProfileStore {
    profile: Arc<RwLock<Profile>>,
}

impl DummyProfileVault {
    pub fn new() -> Self {
        let profile_id = "Iez21JXEtMzXjbCK6BAYFU9ewX".parse::<ProfileId>().unwrap();
        Self { profile_id }
    }
}

impl DummyProfileStore {
    pub fn new(
        vault: &DummyProfileVault,
        addr: &SocketAddr,
        timeout: Duration,
    ) -> std::io::Result<Self> {
        let tcp_stream = TcpStream::connect_timeout(addr, timeout)?;
        // TODO make timeouts configurable
        tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        tcp_stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let tcp_stream_clone = tcp_stream.try_clone()?;
        let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);
        let id = &vault.profile_id;
        let profile = RpcProfile::new(id, Arc::new(Mutex::new(rpc)));
        Ok(Self {
            profile: Arc::new(RwLock::new(profile)),
        })
    }
}

impl ProfileStore for DummyProfileStore {
    fn get(&self, id: &ProfileId) -> Option<Arc<RwLock<Profile>>> {
        Some(self.profile.clone())
    }
    fn create(&self, id: &ProfileId) -> Fallible<Arc<RwLock<Profile>>> {
        unimplemented!()
    }
    fn remove(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}

impl ProfileVault for DummyProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>> {
        let active_opt = self.get_active()?;
        Ok(vec![active_opt.unwrap()])
    }

    fn create_id(&self) -> Fallible<ProfileId> {
        Ok(self.get_active()?.unwrap())
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        Ok(Some(self.profile_id.clone()))
    }
    fn set_active(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
