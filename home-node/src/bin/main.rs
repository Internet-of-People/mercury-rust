#![allow(unused)]
extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;


use std::rc::Rc;

use futures::{Future, Stream};
use tokio_core::reactor;
use tokio_core::net::TcpListener;
use tokio_io::AsyncRead;

use mercury_home_protocol::*;
use mercury_home_node::*;



fn main()
{
    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    use std::net::ToSocketAddrs;
    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind socket");

    let home = Box::new( server::HomeServer::new() );
    let dispatcher = protocol_capnp::HomeDispatcherCapnProto::new(home);
    let home_capnp = mercury_capnp::home::ToClient::new(dispatcher)
        .from_server::<::capnp_rpc::Server>();

    println!("Waiting for clients");
    let handle1 = handle.clone();
    let done = socket.incoming().for_each(move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");
        try!(socket.set_nodelay(true));
        let (reader, writer) = socket.split();
        let handle = handle1.clone();

        let network = capnp_rpc::twoparty::VatNetwork::new( reader, writer,
            capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default() );

        let rpc_system = capnp_rpc::RpcSystem::new( Box::new(network), Some( home_capnp.clone().client ) );

        handle.spawn(rpc_system.map_err(|_| ()));
        Ok(())
    } );

    core.run(done).unwrap();
}
