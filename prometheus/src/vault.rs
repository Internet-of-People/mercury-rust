//use std::collections::HashMap;
use std::net::{SocketAddr, TcpStream};
use std::sync::{Arc, RwLock};
use std::time::Duration;

//use failure::{bail, Fallible};

//use morpheus_keyvault::*;
use morpheus_storage::*;
//use crate::types::{Link, PublicKey, Signature};



// TODO several functions should return Result<> instead, but that needs building our own proper error data structures
pub trait ProfileVault
{
    fn list(&self) -> Vec<ProfileId>; // TODO should this return an iterator instead?
    fn get(&self, id: &ProfileId) -> Option< Arc<RwLock<Profile>> >; // TODO or should list_profiles() return Vec<Profile> and drop this function?
    fn create(&self) -> Arc<RwLock<Profile>>;
    // TODO what does this mean? Purge related metadata from local storage plus don't show it in the list,
    //      or maybe also delete all links/follows with other profiles
    fn remove(&self, id: &ProfileId);

    fn get_active(&self) -> Option<ProfileId>;
    fn set_active(&self, id: &ProfileId) -> bool;
}



pub struct DummyProfileVault
{
    profile: Arc<RwLock<Profile>>,
}

impl DummyProfileVault
{
    pub fn new(addr: &SocketAddr, timeout: Duration) -> std::io::Result<Self>
    {
        let tcp_stream = TcpStream::connect_timeout(addr, timeout)?;
        // TODO make timeouts configurable
        tcp_stream.set_read_timeout( Some( Duration::from_secs(5) ) )?;
        tcp_stream.set_write_timeout( Some( Duration::from_secs(5) ) )?;
        let tcp_stream_clone = tcp_stream.try_clone()?;
        let rpc = MsgPackRpc::new(tcp_stream, tcp_stream_clone);
        let profile = RpcProfile::new( ProfileId{id: vec![42]}, rpc );
        Ok( Self{ profile: Arc::new( RwLock::new(profile) ) } )
    }
}

impl ProfileVault for DummyProfileVault
{
    fn list(&self) -> Vec<ProfileId> { vec![ self.get_active().unwrap() ] }
    fn get(&self, id: &ProfileId) -> Option< Arc<RwLock<Profile>> >
        { Some( self.profile.clone() ) }
    fn create(&self) -> Arc<RwLock<Profile>> { unimplemented!() }
    fn remove(&self, id: &ProfileId) { unimplemented!() }

    fn get_active(&self) -> Option<ProfileId> { Some( self.profile.read().unwrap().id().to_owned() ) }
    fn set_active(&self, id: &ProfileId) -> bool { unimplemented!() }
}



// TODO remove this when something else works and can be tested
pub struct FailingProfileVault {}

impl ProfileVault for FailingProfileVault
{
    fn list(&self) -> Vec<ProfileId> { unimplemented!() }
    fn get(&self, id: &ProfileId) -> Option< Arc<RwLock<Profile>> > { unimplemented!() }
    fn create(&self) -> Arc<RwLock<Profile>> { unimplemented!() }
    fn remove(&self, id: &ProfileId) { unimplemented!() }

    fn get_active(&self) -> Option<ProfileId> { unimplemented!() }
    fn set_active(&self, id: &ProfileId) -> bool { unimplemented!() }
}
