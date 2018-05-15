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


use futures::Stream;
use tokio_core::reactor;
use tokio_core::net::TcpListener;

use mercury_home_node::*;
use mercury_home_node::crypto::{CompositeValidator, Ed25519Validator, MultiHashProfileValidator};
use mercury_storage::async::{ModularHashSpace, imp::InMemoryStore};



fn main()
{
    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    use std::net::ToSocketAddrs;
    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind socket");

    println!("Waiting for clients");
    let handle1 = handle.clone();
    let done = socket.incoming().for_each(move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");

        // TODO use persistent storage
        //let distributed_storage = Box::new( Ipfs::new( "localhost", 5001, &handle1.clone() )? )
        let distributed_storage = Box::new( InMemoryStore::new() );
        let local_storage = Box::new( InMemoryStore::new() );
        let profile_validator = MultiHashProfileValidator::new();
        let signature_validator = Ed25519Validator::new();
        let validator = Box::new( CompositeValidator::new(profile_validator, signature_validator) );
        let home = Box::new( server::HomeServer::new(distributed_storage, local_storage, validator) );
        protocol_capnp::HomeDispatcherCapnProto::dispatch_tcp( home, socket, handle1.clone() );
        Ok( () )
    } );

    core.run(done).unwrap();
}
