extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate tokio_core;
extern crate tokio_io;



#[test]
fn test_events()
{
    use std::net::ToSocketAddrs;
    use std::rc::Rc;

    use futures::{Stream};
    use tokio_core::net::{TcpListener, TcpStream};
    use tokio_core::reactor;

    //use mercury_home_protocol::*;
    use mercury_connect::HomeContext;
    use mercury_connect::mock::{DummyHome, Signo, make_home_profile};
    use mercury_connect::protocol_capnp::HomeClientCapnProto;
    use mercury_home_node::protocol_capnp::HomeDispatcherCapnProto;



    let mut reactor = reactor::Core::new().unwrap();

    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");

    let handle1 = reactor.handle();
    let server_socket = TcpListener::bind( &addr, &reactor.handle() ).expect("Failed to bind socket");
    let server_fut = server_socket.incoming().for_each( move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");

        let home = Box::new( DummyHome::new("ping_reply_msg") );
        // let home = Box::new( server::HomeServer::new() );
        HomeDispatcherCapnProto::dispatch_tcp( home, socket, handle1.clone() );
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