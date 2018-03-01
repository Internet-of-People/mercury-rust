extern crate futures;
extern crate multiaddr;
extern crate tokio_core;

use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr};
use tokio_core::reactor;
use tokio_core::net::TcpStream;



pub enum SearchProfileError
{
    TODO // TODO
}

pub enum SearchAddressError
{
    TODO // TODO
}

pub enum OpenAddressError
{
    TODO // TODO
}

pub enum ConnectToProfileError
{
    LookupFailed(SearchAddressError),
    ConnectFailed(OpenAddressError),
    Other(Box<std::error::Error>),
}



// TODO for Tor, I2P and similar cases, the TcpStream return type might not work out,
// maybe we'll need something like (Stream<u8>,Sink<u8>) or something similar
pub fn open(multiaddr: Multiaddr) ->
    Box< Future<Item=TcpStream, Error=OpenAddressError> >
{
    Box::new( future::err(OpenAddressError::TODO) )
}


type ProfileId = String;
type ServiceId = String;

pub struct Profile
{
    id:         ProfileId,
// TODO
//    pub_key:    Vec<u8>,
//    priv_key:   Vec<u8>,
//    metadata:   ???,
}

impl Profile
{
    pub fn new(id: ProfileId) -> Self
        { Self{ id: id } }

    // TODO
    // pub fn register(profile_server: ???) -> ???

    // TODO
    // pub fn listen(???) -> ???

    pub fn find_profile( /* TODO what filter criteria should we have here? */ ) ->
        Box< Future<Item=Profile, Error=SearchProfileError> >
    {
        Box::new( future::err(SearchProfileError::TODO) )
    }

    // TODO should we use Stream<Multiaddr> here instead of Vec<>?
    pub fn find_contacts(&self) ->
        Box< Future<Item=Vec<Multiaddr>, Error=SearchAddressError> >
    {
        Box::new( future::err(SearchAddressError::TODO) )
    }

    pub fn open(&self) ->
        Box< Future<Item=TcpStream, Error=ConnectToProfileError>>
    {
        let result = self.find_contacts()
            .map_err( |e| ConnectToProfileError::LookupFailed(e) )
            .and_then( |contacts|
            {
                for contact in contacts
                {
                }
                future::err( ConnectToProfileError::LookupFailed(SearchAddressError::TODO) )
            } );

        Box::new(result)
    }
}



// TODO
pub enum ErrorToBeSpecified { TODO, }

pub trait IdentityStore
{
    fn connect(addr: Multiaddr) ->
        Box< Future<Item=Self, Error=ErrorToBeSpecified> >;

    fn register(&self, prof: Profile) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    fn remove(&self, prof: Profile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}


pub trait HomeConnection
{
    fn connect(addr: Multiaddr, profiles: Vec<Profile>, services: Vec<ServiceId>) ->
        Box< Future<Item=Self, Error=ErrorToBeSpecified> >;

    fn stream(&self, srv: ServiceId) -> TcpStream;

    fn register(&self, services: Vec<ServiceId>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn remove(&self, services: Vec<ServiceId>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn close(&self);
}


#[cfg(test)]
mod tests
{
    #[test]
    fn test_register_profile()
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
    fn test_connect_multiaddr()
    {
        // TODO
    }

    #[test]
    fn test_connect_profile()
    {
        // TODO
    }

    #[test]
    fn test_listen()
    {
        // TODO
    }
}
