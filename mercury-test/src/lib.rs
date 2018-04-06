extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_common;
extern crate mercury_sdk;
extern crate mercury_profile_server;
extern crate tokio_core;
extern crate tokio_io;



#[test]
fn test_events()
{
    use mercury_common::*;
    use mercury_sdk::HomeContext;
    use mercury_sdk::mock::{DummyHome, Signo, make_home_profile};
    use mercury_sdk::protocol_capnp::HomeClientCapnProto;
    use mercury_profile_server::protocol_capnp::HomeDispatcherCapnProto;

    use futures::{Future, Stream};
    use std::net::ToSocketAddrs;
    use std::rc::Rc;
    use tokio_core::net::{TcpListener, TcpStream};
    use tokio_core::reactor;
    use tokio_io::AsyncRead;


    let mut reactor = reactor::Core::new().unwrap();

    let home = DummyHome::new("ping_reply_msg");
    let dispatcher = HomeDispatcherCapnProto::new( Box::new(home) );

    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let server_home = mercury_capnp::home::ToClient::new(dispatcher)
        .from_server::<::capnp_rpc::Server>();

    let handle1 = reactor.handle();
    let server_socket = TcpListener::bind( &addr, &reactor.handle() ).expect("Failed to bind socket");
    let server_fut = server_socket.incoming().for_each( move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");
        try!( socket.set_nodelay(true) );
        let (reader, writer) = socket.split();
        let handle = handle1.clone();

        let network = capnp_rpc::twoparty::VatNetwork::new( reader, writer,
            capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default() );
        let rpc_system = capnp_rpc::RpcSystem::new( Box::new(network), Some( server_home.clone().client ) );
        handle.spawn( rpc_system.map_err( |_| () ) );
        Ok( () )
    } );

    let tcp_fut = TcpStream::connect( &addr, &reactor.handle() );
    let tcp_stream = reactor.run(tcp_fut).unwrap();

    let signer = Rc::new( Signo::new("privatekey") );
    let home_profile = make_home_profile("home_address", "home_profile", "home_public_key");
    let home_ctx = Box::new( HomeContext::new(signer, &home_profile) );
    let client = HomeClientCapnProto::new_tcp( tcp_stream, home_ctx, reactor.handle() );


//    let res = reactor.run(home_session);
    assert!(true);
}