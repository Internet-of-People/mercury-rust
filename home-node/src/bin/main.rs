extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate mercury_storage;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;


use std::rc::Rc;

use futures::Stream;
use tokio_core::reactor;
use tokio_core::net::TcpListener;

use mercury_home_protocol::*;
use mercury_home_node::crypto::*;
use mercury_home_node::server::*;
use mercury_home_node::protocol_capnp;
use mercury_storage::async::{ModularHashSpace, imp::InMemoryStore};



fn main()
{
    // TODO parse a configuration and set up keys accordingly, possibly from hardware wallet
    let secret_key = PrivateKey( b"\x83\x3F\xE6\x24\x09\x23\x7B\x9D\x62\xEC\x77\x58\x75\x20\x91\x1E\x9A\x75\x9C\xEC\x1D\x19\x75\x5B\x7D\xA9\x01\xB9\x6D\xCA\x3D\x42".to_vec() );
    let public_key = PublicKey( b"\xEC\x17\x2B\x93\xAD\x5E\x56\x3B\xF4\x93\x2C\x70\xE1\x24\x50\x34\xC3\x54\x67\xEF\x2E\xFD\x4D\x64\xEB\xF8\x19\x68\x34\x67\xE2\xBF".to_vec() );
    let signer = Rc::new( Ed25519Signer::new(&secret_key, &public_key)
        .expect("Failed to initialize server identity"));

    let profile_validator = MultiHashProfileValidator::new();
    let signature_validator = Ed25519Validator::new();
    let validator = Rc::new( CompositeValidator::new(profile_validator, signature_validator) );

    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    use std::net::ToSocketAddrs;
    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind socket");

    println!("Waiting for clients");
    let handle_clone = handle.clone();
    let signer_clone = signer.clone();
    let validator_clone = validator.clone();
    let done = socket.incoming().for_each(move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");

        // TODO fill this in properly for each connection based on TLS authentication info
        let client_pub_key = PublicKey( b"TestPublicKey: TODO implement filling this in properly with TLS authentication info".to_vec() );
        let client_profile_id = ProfileId( b"TestClientId: TODO implement filling this in properly with TLS authentication info".to_vec() );
        let context = Box::new( ClientContext::new( signer_clone.clone(), client_pub_key, client_profile_id ) );

        // TODO use persistent storages both for local and distributed
        //let distributed_storage = Box::new( Ipfs::new( "localhost", 5001, &handle1.clone() )? )
        let distributed_storage = Box::new( InMemoryStore::new() );
        let local_storage = Box::new( InMemoryStore::new() );

        let home = Box::new( HomeServer::new(context, validator.clone(), distributed_storage, local_storage) );
        protocol_capnp::HomeDispatcherCapnProto::dispatch_tcp( home, socket, handle_clone.clone() );
        Ok( () )
    } );

    core.run(done).unwrap();
}
