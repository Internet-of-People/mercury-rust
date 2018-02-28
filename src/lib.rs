extern crate futures;
extern crate multiaddr;
extern crate tokio_core;

use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr};
use tokio_core::reactor;
use tokio_core::net::TcpStream;



type ProfileId = Vec<u8>;

enum ProfileLookupError
{
    TODO // TODO
}

fn find_profile( /* TODO what filter criteria should we have here? */ ) ->
Box< Future<Item=ProfileId, Error=ProfileLookupError> >
{
    Box::new( future::err(ProfileLookupError::TODO) )
}



enum ContactLookupError
{
    TODO // TODO
}

// TODO should we use Stream<Multiaddr> here instead of Vec<>?
fn find_contacts(profile_id: ProfileId) ->
Box< Future<Item=Vec<Multiaddr>, Error=ContactLookupError> >
{
    Box::new( future::err(ContactLookupError::TODO) )
}



enum AddressConnectError
{
    TODO // TODO
}

// TODO for Tor, I2P and similar cases, the TcpStream return type might not work out,
// maybe we'll need something like (Stream<u8>,Sink<u8>) or something similar
fn connect_address(multiaddr: Multiaddr) ->
Box< Future<Item=TcpStream, Error=AddressConnectError> >
{
    Box::new( future::err(AddressConnectError::TODO) )
}



enum ProfileConnectError
{
    ContactLookup(ContactLookupError),
    Connect(AddressConnectError),
    Other(Box<std::error::Error>),
}

fn connect_profile(profile_id: ProfileId) ->
    Box< Future<Item=TcpStream, Error=ProfileConnectError>>
{
    let result = find_contacts(profile_id)
        .map_err( |e| ProfileConnectError::ContactLookup(e) )
        .and_then( |contacts| {
            for contact in contacts
            {
            }
            future::err( ProfileConnectError::ContactLookup(ContactLookupError::TODO) )
        } );

    Box::new(result)
}



#[cfg(test)]
mod tests
{
    #[test]
    fn it_works()
    {
        // TODO
    }
}
