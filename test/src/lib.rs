extern crate capnp;
extern crate capnp_rpc;
extern crate ed25519_dalek;
extern crate futures;
extern crate mercury_storage;
extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate mercury_storage;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_stdin_stdout;
extern crate multiaddr;
extern crate multihash;
extern crate rand;
extern crate sha2;
extern crate tokio_stdin_stdout;

use std::{cell::RefCell, rc::Rc};

use rand::OsRng;
use sha2::Sha512;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_protocol::crypto::*;
use mercury_home_node::server::HomeServer;
use mercury_storage::async::imp::InMemoryStore;


pub mod dummy; // TODO this will not be needed with real components ready and tested

#[cfg(test)]
pub mod connect;
#[cfg(test)]
pub mod home;



pub fn generate_keypair() -> (PrivateKey, PublicKey) {
    let mut csprng: OsRng = OsRng::new().unwrap();
    let secret_key = ed25519_dalek::SecretKey::generate(&mut csprng);
    let public_key = ed25519_dalek::PublicKey::from_secret::<Sha512>(&secret_key);
    (PrivateKey::from(secret_key), PublicKey::from(public_key))
}


pub fn generate_ownprofile(facet: ProfileFacet, private_data: Vec<u8>)
    -> (OwnProfile, Ed25519Signer)
{
    let (private_key, public_key) = generate_keypair();
    let signer = Ed25519Signer::new(&private_key, &public_key).expect("TODO: this should not be able to fail");
    let profile = Profile::new( &(&public_key).into(), &public_key, &vec![facet] );
    //let profile = Profile::new( &ProfileId::from(&public_key), &public_key, vec![facet] );
    let own_profile = OwnProfile::new(&profile, &private_data);
    (own_profile, signer)
}

pub fn generate_profile(facet: ProfileFacet) -> (Profile, Ed25519Signer)
{
    let (own_profile, signer) = generate_ownprofile(facet, vec![]);
    (own_profile.profile, signer)
}



pub fn default_home_server(handle: &reactor::Handle) -> HomeServer {
    HomeServer::new( handle,
        Rc::new( CompositeValidator::default() ),
        Rc::new( RefCell::new( InMemoryStore::new() ) ),
        Rc::new( RefCell::new( InMemoryStore::new() ) ),
    )
}


//TODO might need to place this to some other place
#[test]
fn profile_serialize_async_key_value_test() {
    use tokio_core;
    use tokio_core::reactor;

    
    let profile = Profile::new(
        &ProfileId("userprofile".into()), 
        &PublicKey("userkey".into()), 
        &vec![]
    );

    let homeprofile = Profile::new_home(
        ProfileId("homeprofile".into()), 
        PublicKey("homekey".into()), 
        String::from("/ip4/127.0.0.1/udp/9876").to_multiaddr().unwrap()
    );

    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : AsyncFileHandler = AsyncFileHandler::new(String::from("./ipfs/homeserverid/")).unwrap();

    let set = storage.set(profile.id.clone(), profile.clone());
    let sethome = storage.set(homeprofile.id.clone(), homeprofile.clone());

    reactor.run(set).unwrap();
    reactor.run(sethome).unwrap();

    let read = storage.get(profile.id.clone());
    let readhome = storage.get(homeprofile.id.clone());

    let res = reactor.run(read).unwrap();
    let reshome = reactor.run(readhome).unwrap();
    assert_eq!(res, profile);
    assert_eq!(reshome, homeprofile);
}