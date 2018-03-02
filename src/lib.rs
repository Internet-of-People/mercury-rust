extern crate futures;
extern crate multiaddr;
extern crate tokio_core;

use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr};
use tokio_core::reactor;
use tokio_core::net::TcpStream;



// TODO
pub enum ErrorToBeSpecified { TODO, }

pub enum SearchProfileError
{
    TODO // TODO
}

pub enum SearchAddressError
{
    TODO // TODO
}

pub enum ConnectAddressError
{
    TODO // TODO
}

pub enum ConnectToContactError
{
    LookupFailed(SearchAddressError),
    ConnectFailed(ConnectAddressError),
    Other(Box<std::error::Error>),
}



// TODO for Tor, I2P and similar cases, the TcpStream return type might not work out,
// maybe we'll need something like (Stream<u8>,Sink<u8>) or something similar
pub fn open(multiaddr: Multiaddr) ->
    Box< Future<Item=TcpStream, Error=ConnectAddressError> >
{
    Box::new( future::err(ConnectAddressError::TODO) )
}


type ProfileId = String;
type ApplicationServiceId = String;

#[derive(Debug, Clone)]
pub struct Profile
{
    id:         ProfileId,
// TODO
//    pub_key:    Vec<u8>???,
//    metadata:   ???,
}

impl Profile
{
    pub fn new(id: &ProfileId) -> Self
        { Self{ id: id.clone() } }

    pub fn list( /* TODO what filter criteria should we have here? */ ) ->
        Box< Future<Item=Vec<Profile>, Error=SearchProfileError> >
    {
        // TODO explore ProfileHosts and query profiles on each of them
        Box::new( future::err(SearchProfileError::TODO) )
    }

    // TODO should we use Stream<Multiaddr> here instead of Vec<>?
    pub fn find_addresses(&self) ->
        Box< Future<Item=Vec<Multiaddr>, Error=SearchAddressError> >
    {
        // TODO explore profile Home and query known addresses for profile
        Box::new( future::err(SearchAddressError::TODO) )
    }
}



#[derive(Debug, Clone)]
pub struct Contact
{
    profile:    Profile,
//    proof_of_handshake: TODO???,
}



#[derive(Debug, Clone)]
pub struct OwnProfile
{
    profile:    Profile,
//    priv_key:   Vec<u8>???,
//    services: Vec<ApplicationServiceId>, ??? should we have the services here ???
}

impl OwnProfile
{
    pub fn new(profile: &Profile, /* TODO priv_key, services? */ ) -> Self
        { Self{ profile: profile.clone() } }

    pub fn pair_with(&self, profile: &Profile) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    // TODO for Tor, I2P and similar cases, the TcpStream return type might not work out,
    pub fn connect_to(&self, contact: &Contact, appsrv: &ApplicationServiceId) ->
        Box< Future<Item=TcpStream, Error=ConnectToContactError>>
    {
        let result = contact.profile.find_addresses()
            .map_err( |e| ConnectToContactError::LookupFailed(e) )
            .and_then( |addrs|
            {
                for addr in addrs
                {
                }
                future::err( ConnectToContactError::ConnectFailed(ConnectAddressError::TODO) )
            } );

        Box::new(result)
    }
}



pub struct ProfileHost {} // TODO

impl ProfileHost
{
    pub fn new(addr: &Multiaddr) ->
        Box< Future<Item=Self, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    pub fn register(&self, prof: &OwnProfile) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    pub fn remove(&self, prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    pub fn find_profile(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Future<Item=Profile, Error=SearchProfileError> >
    {
        Box::new( future::err(SearchProfileError::TODO) )
    }

    pub fn find_addresses(&self, profile: &Profile) ->
        Box< Future<Item=Vec<Multiaddr>, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    // TODO is this really different from Home::new() where identity must be proven?
    pub fn claim(&self, profile: &Profile, /* TODO what other params to prove ownership? */ ) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}



pub trait Home
{
    fn new(addr: &Multiaddr, profiles: &[OwnProfile], services: &[ApplicationServiceId]) ->
        Box< Future<Item=Self, Error=ErrorToBeSpecified> >;

    // TODO this should be appsrv-level but then raw TcpStream might not work
    //      (all application services are supposed to share a single connection to spare battery),
    //      we'll probably need something like Stream<MessageType> here
    fn connection(&self) -> TcpStream;

    // TODO should we explicitly handle close?
    fn close(self);

    fn register(&self, services: &[ApplicationServiceId]) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn remove(&self, services: &[ApplicationServiceId]) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



#[cfg(test)]
mod tests
{
    use super::*;
    use multiaddr::ToMultiaddr;


    struct TestSetup
    {
        reactor: reactor::Core,
    }

    impl TestSetup
    {
        fn new() -> Self
        {
            Self{ reactor: reactor::Core::new().unwrap() }
        }
    }


    #[test]
    fn test_connect_multiaddr()
    {
        let mut setup = TestSetup::new();
        let addr = "/ip4/127.0.0.1/tcp/12345".to_multiaddr().unwrap();
        let connect_fut = open(addr);
        let result = setup.reactor.run(connect_fut);
        // TODO assert!( result.TODO );
    }


    #[test]
    fn test_register_profile()
    {
        let mut setup = TestSetup::new();
        let ownprof = OwnProfile::new( &Profile::new( &"OwnProfileId".to_string() ) );
        let addr = "/ip4/127.0.0.1/tcp/23456".to_multiaddr().unwrap();
        let register_fut = ProfileHost::new(&addr)
            .and_then( move |host|
                { host.register(&ownprof) } );
        let result = setup.reactor.run(register_fut);
        // TODO assert!( result.TODO );
    }


    #[test]
    fn test_claim_profile()
    {
        // TODO
    }

    #[test]
    fn test_pair_profiles()
    {
        // TODO
    }

    #[test]
    fn test_lookup_profiles()
    {
        // TODO
    }

    #[test]
    fn test_lookup_contacts()
    {
        // TODO
    }

    #[test]
    fn test_connect_profile_service()
    {
        // TODO
    }

    #[test]
    fn test_appservice_connect()
    {
        // TODO
    }

    #[test]
    fn test_appservice_listen()
    {
        // TODO
    }
}
