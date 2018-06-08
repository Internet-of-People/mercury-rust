extern crate futures;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate multiaddr;
//extern crate multihash;
extern crate tokio_core;

use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::time::Duration;

use futures::Future;
use tokio_core::net::TcpStream;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_protocol::crypto::*;
use mercury_connect::HomeContext;
use mercury_connect::protocol_capnp::*;



fn main()
{
    log4rs::init_file( "log4rs.yml", Default::default() ).unwrap();

//        use mercury_storage::common::Hasher;
//        let test = b"\xD7\x5A\x98\x01\x82\xB1\x0A\xB7\xD5\x4B\xFE\xD3\xC9\x64\x07\x3A\x0E\xE1\x72\xF3\xDA\xA6\x23\x25\xAF\x02\x1A\x68\xF7\x07\x51\x1A".to_vec();
//        let hasher = mercury_storage::common::imp::MultiHasher::new(multihash::Hash::Keccak256);
//        let hash = hasher.get_hash(&test).unwrap();
//        let mapped = hash.iter().map( |x| format!("\\x{:X?}", x) ).collect::<Vec<_>>().join("");
//        debug!("HASH: {:?}", mapped );

    let mut reactor = reactor::Core::new().unwrap();

    let client_private_key = PrivateKey( b"\x9D\x61\xB1\x9D\xEF\xFD\x5A\x60\xBA\x84\x4A\xF4\x92\xEC\x2C\xC4\x44\x49\xC5\x69\x7B\x32\x69\x19\x70\x3B\xAC\x03\x1C\xAE\x7F\x60".to_vec() );
    let client_public_key = PublicKey( b"\xD7\x5A\x98\x01\x82\xB1\x0A\xB7\xD5\x4B\xFE\xD3\xC9\x64\x07\x3A\x0E\xE1\x72\xF3\xDA\xA6\x23\x25\xAF\x02\x1A\x68\xF7\x07\x51\x1A".to_vec() );
    let prof_id = ProfileId( b"\x1B\x20\x9E\xE7\xC0\x9B\x84\x64\x02\x8B\x2C\xD4\x06\xF7\xF7\xCC\x70\xAD\xC6\x36\x59\xB5\xD3\x76\x71\xDC\x2B\x58\x8D\xB3\x24\x46\x68\x4A".to_vec() );
    let signer = Rc::new( Ed25519Signer::new(&client_private_key, &client_public_key).unwrap() );

    //let signer = Rc::new( DummySigner{ prof_id: prof_id.clone(), pub_key: PublicKey(Vec::new()) } );
    let home_id = ProfileId( b"\xEC\x17\x2B\x93\xAD\x5E\x56\x3B\xF4\x93\x2C\x70\xE1\x24\x50\x34\xC3\x54\x67\xEF\x2E\xFD\x4D\x64\xEB\xF8\x19\x68\x34\x67\xE2\xBF".to_vec() );
    let home_facet = HomeFacet{ addrs: Vec::new(), data: Vec::new() };
    let home_prof = Profile::new( &home_id,
        &PublicKey( "HomePubKey".as_bytes().to_owned() ),
        &[ ProfileFacet::Home(home_facet) ] );
    let home_ctx = Box::new( HomeContext::new(signer, &home_prof) );

    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let handle = reactor.handle();
    let handle2 = reactor.handle();
    let handle3 = reactor.handle();
    let test_fut = TcpStream::connect( &addr, &reactor.handle() )
        .map_err( |e| ErrorToBeSpecified::TODO( format!("temporaty_test_capnproto connect: {:?}", e) ) )
        .and_then( move |tcp_stream|
        {
            let home = HomeClientCapnProto::new_tcp(tcp_stream, home_ctx, handle);
            //home.load(&prof_id)
            home.login(prof_id)
        } )
        .and_then( |session| reactor::Timeout::new( Duration::from_secs(5), &handle2 ).unwrap()
            .map( move |_| session )
            .map_err( |e| ErrorToBeSpecified::TODO( format!("temporary_test_capnproto session: {:?}", e) ) ) )
        .and_then( |session| session.ping("hahoooo") )
        .and_then( |pong|
        {
            debug!("Got pong {}", pong);
            reactor::Timeout::new( Duration::from_secs(5), &handle3 ).unwrap()
                .map( move |_| pong )
                .map_err( |e| ErrorToBeSpecified::TODO( format!("temporary_test_capnproto can't play ping-pong {:?}", e) ) )
        } );

    let pong = reactor.run(test_fut);
    debug!("Response: {:?}", pong);

    let handle = reactor.handle();
    let result = reactor.run( reactor::Timeout::new( Duration::from_secs(5), &handle ).unwrap() );
    info!("Client result {:?}", result);
}