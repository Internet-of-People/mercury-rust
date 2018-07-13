extern crate futures;
#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate clap;

extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate multiaddr;
//extern crate multihash;
extern crate tokio_core;


use clap::App;

use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::time::Duration;

use multiaddr::ToMultiaddr;

use futures::Future;
use tokio_core::net::TcpStream;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_protocol::crypto::*;
use mercury_connect::*;


fn main()
{
    log4rs::init_file( "log4rs.yml", Default::default() ).unwrap();

//        use mercury_storage::common::Hasher;
//        let test = b"\xD7\x5A\x98\x01\x82\xB1\x0A\xB7\xD5\x4B\xFE\xD3\xC9\x64\x07\x3A\x0E\xE1\x72\xF3\xDA\xA6\x23\x25\xAF\x02\x1A\x68\xF7\x07\x51\x1A".to_vec();
//        let hasher = mercury_storage::common::imp::MultiHasher::new(multihash::Hash::Keccak256);
//        let hash = hasher.get_hash(&test).unwrap();
//        let mapped = hash.iter().map( |x| format!("\\x{:X?}", x) ).collect::<Vec<_>>().join("");
//        debug!("HASH: {:?}", mapped );

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let client_private_key = PrivateKey( b"\x9D\x61\xB1\x9D\xEF\xFD\x5A\x60\xBA\x84\x4A\xF4\x92\xEC\x2C\xC4\x44\x49\xC5\x69\x7B\x32\x69\x19\x70\x3B\xAC\x03\x1C\xAE\x7F\x60".to_vec() );
    //let client_public_key = PublicKey( b"\xD7\x5A\x98\x01\x82\xB1\x0A\xB7\xD5\x4B\xFE\xD3\xC9\x64\x07\x3A\x0E\xE1\x72\xF3\xDA\xA6\x23\x25\xAF\x02\x1A\x68\xF7\x07\x51\x1A".to_vec() );
    //let client_profile_id = ProfileId( b"\x1B\x20\x9E\xE7\xC0\x9B\x84\x64\x02\x8B\x2C\xD4\x06\xF7\xF7\xCC\x70\xAD\xC6\x36\x59\xB5\xD3\x76\x71\xDC\x2B\x58\x8D\xB3\x24\x46\x68\x4A".to_vec() );
    let client_signer = Rc::new( Ed25519Signer::new(&client_private_key).unwrap() );
    let client_facet = ProfileFacet::Persona(PersonaFacet {homes: vec![], data: vec![]});
    let client_profile = Profile::new(&client_signer.profile_id(), &client_signer.public_key(), &client_facet);
    let client_own_profile = OwnProfile::new(&client_profile, &vec![]);

    // server details has to be taken from the command line
    // we need 3 pieces of information
    // (1) ProfileId
    // (2) Public key hash
    // (3) Address (since we don't yet have access to ipfs)
    let server_key = PublicKey(matches.value_of("server-key").unwrap().as_bytes().to_vec());
    let server_id = ProfileId(matches.value_of("server-id").unwrap().as_bytes().to_vec());
    let addr = matches.value_of("server-addr").unwrap().to_multiaddr().expect("Failed to parse address");    

    let mut reactor = reactor::Core::new().unwrap();
    let handle = reactor.handle();
    let handle2 = reactor.handle();
    let handle3 = reactor.handle();
    let client_signer_clone = client_signer.clone();

    let mut profile_store = SimpleProfileRepo::new();

    let home_profile = Profile::new_home(server_id.clone(), server_key, addr);
    profile_store.insert(home_profile);

    let home_connector = SimpleTcpHomeConnector::new(reactor.handle());

    let profile_gw = ProfileGatewayImpl::new(client_signer_clone, Rc::new(profile_store),  Rc::new(home_connector));







/*
    let test_fut = TcpStream::connect( &addr, &reactor.handle() )
        .map_err( |e| ErrorToBeSpecified::TODO( format!("temporaty_test_capnproto connect: {:?}", e) ) )
        .and_then( move |socket|
        {
            handshake::temp_tcp_handshake_until_tls_is_implemented( socket, client_signer_clone )
        } )
        .map( move |(reader, writer, home_context)|
        {
            let home_id = home_context.peer_id().clone();
            let home_client = HomeClientCapnProto::new(reader, writer, home_context, handle);
            (home_id, home_client)
        } )
        .and_then( |(home_id, home)|
        {
            let halfproof = RelationHalfProof::new("home", &home_id, &*client_signer);
            home.register(client_own_profile, halfproof, None)
                .map( |_own_profile| home )
                .map_err( |(_own_profile, e)| e )
        } )
        .and_then( |home| home.login( client_signer.profile_id() ) )
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


*/

    let test_fut = profile_gw.connect_home(&server_id.clone())
        .and_then(move |home| {
            info!("connected, logging in");
            home.login(&server_id.clone())
        })
        .and_then(|session| {
            info!("session created, sending ping");
            session.ping("hahoooo")
        })
        .map(|pong| {
            info!("received pong");
            pong
        })
        .map_err(|_| ErrorToBeSpecified::TODO(String::from("profile gateway failed to login()")));

    let pong = reactor.run(test_fut);
    
    debug!("Response: {:?}", pong);

    let handle = reactor.handle();
    let result = reactor.run( reactor::Timeout::new( Duration::from_secs(5), &handle ).unwrap() );
    info!("Client result {:?}", result);
}