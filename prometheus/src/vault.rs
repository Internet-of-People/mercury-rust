//use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use failure::{bail, Fallible};

//use morpheus_keyvault::*;
use morpheus_storage::*;
//use crate::types::{Link, PublicKey, Signature};


pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>; // TODO should this return an iterator instead?
    fn get(&self, id: &ProfileId) -> Option<Arc<RwLock<Profile>>>; // TODO or should list_profiles() return Vec<Profile> and drop this function?
    fn create(&self) -> Fallible<Arc<RwLock<Profile>>>;
    // TODO what does this mean? Purge related metadata from local storage plus don't show it in the list,
    //      or maybe also delete all links/follows with other profiles
    fn remove(&self, id: &ProfileId) -> Fallible<()>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&self, id: &ProfileId) -> Fallible<()>;
}

pub struct DummyProfileVault {
    profile: Arc<RwLock<Profile>>,
}

impl DummyProfileVault {
    pub fn new(addr: &SocketAddr, timeout: Duration) -> std::io::Result<Self> {
        let tcp_stream = TcpStream::connect_timeout(addr, timeout)?;
        // TODO make timeouts configurable
        tcp_stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        tcp_stream.set_write_timeout(Some(Duration::from_secs(5)))?;
        let tcp_stream_clone = tcp_stream.try_clone()?;
        let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);
        let profile = RpcProfile::new(ProfileId { id: vec![42] }, rpc);
        Ok(Self {
            profile: Arc::new(RwLock::new(profile)),
        })
    }
}

impl ProfileVault for DummyProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>> {
        let active_opt = self.get_active()?;
        Ok(vec![active_opt.unwrap()])
    }
    fn get(&self, id: &ProfileId) -> Option< Arc<RwLock<Profile>> > {
        Some(self.profile.clone())
    }
    fn create(&self) -> Fallible< Arc<RwLock<Profile>> > {
        unimplemented!()
    }
    fn remove(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        let profile_id = match self.profile.read() {
            Ok(profile) => profile.id().to_owned(),
            Err(e) => bail!(
                "Implementation error: failed to get read access to selected profile: {}",
                e
            ),
        };
        Ok(Some(profile_id))
    }
    fn set_active(&self, id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
}
