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

    use futures::{select_ok, Future, Stream};
    use tokio_core::net::{TcpListener, TcpStream};
    use tokio_core::reactor;

    use mercury_home_protocol::*;
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

    let handle2 = reactor.handle();
    let client_fut = TcpStream::connect( &addr, &reactor.handle() )
        .map_err( |_e| ErrorToBeSpecified::TODO)
        .and_then( |tcp_stream|
        {
            let signer = Rc::new( Signo::new("privatekey") );
            let my_profile = signer.prof_id().clone();
            let home_profile = make_home_profile("home_address", "home_profile", "home_public_key");
            let home_ctx = Box::new( HomeContext::new(signer, &home_profile) );
            let client = HomeClientCapnProto::new_tcp( tcp_stream, home_ctx, handle2 );
            client.login(my_profile) // TODO maybe we should require only a reference in login()
        } )
        .map( |session|
        {
            session.events() //.for_each( |event| () )
        } );

    let futs = [ Box::new( server_fut.map_err( |_e| () ) ) as Box< Future<Item=(),Error=()> >,
                 Box::new( client_fut.map( |_session| () ).map_err( |_e| () ) ) ];
//    let both_fut = select_ok( futs.iter() ); // **i as &Future<Item=(),Error=()> ) );
//    let result = reactor.run(both_fut);
}