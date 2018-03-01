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



pub struct Contact
{
    profile:    Profile,
//    proof_of_handshake: TODO???,
}



pub struct OwnedProfile
{
    profile:    Profile,
//    priv_key:   Vec<u8>???,
}

impl OwnedProfile
{
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



pub trait ProfileHost
{
    fn open(addr: &Multiaddr) ->
        Box< Future<Item=Self, Error=ErrorToBeSpecified> >;

    fn register(&self, prof: &OwnedProfile) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    fn remove(&self, prof: &OwnedProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn find_profile(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Future<Item=Profile, Error=SearchProfileError> >;

    fn find_addresses(&self, profile: &Profile) ->
        Box< Future<Item=Vec<Multiaddr>, Error=ErrorToBeSpecified> >;

    fn claim(&self, profile: &Profile, /* TODO what other params to prove ownership? */ ) ->
        Box< Future<Item=OwnedProfile, Error=ErrorToBeSpecified> >;
}


pub trait Home
{
    fn open(addr: &Multiaddr, profiles: &[OwnedProfile], services: &[ApplicationServiceId]) ->
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
    #[test]
    fn test_connect_multiaddr()
    {
        // TODO
    }

    #[test]
    fn test_register_profile()
    {
        // TODO
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
    fn test_service_listen()
    {
        // TODO
    }
}
