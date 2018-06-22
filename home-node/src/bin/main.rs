extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate mercury_storage;
extern crate multiaddr;
//extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;


use std::cell::RefCell;
use std::rc::Rc;

use futures::{Future, Stream};
use tokio_core::{reactor, net::TcpListener};

use mercury_home_protocol::{*, crypto::*, handshake};
use mercury_home_node::{config::*, server::*, protocol_capnp};
use mercury_storage::async::imp::InMemoryStore;



fn main()
{
    log4rs::init_file( "log4rs.yml", Default::default() ).unwrap();

    let _config = FileCliParser::parse_config();
    //println!( "Profile Id: {:?}", config.args.value_of("profile_id") );

    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    // TODO parse a configuration and set up keys accordingly, possibly from hardware wallet
    //      until then for some test keys see https://github.com/tendermint/signatory/blob/master/src/ed25519/test_vectors.rs
    let secret_key = PrivateKey( b"\x83\x3F\xE6\x24\x09\x23\x7B\x9D\x62\xEC\x77\x58\x75\x20\x91\x1E\x9A\x75\x9C\xEC\x1D\x19\x75\x5B\x7D\xA9\x01\xB9\x6D\xCA\x3D\x42".to_vec() );
    let public_key = PublicKey( b"\xEC\x17\x2B\x93\xAD\x5E\x56\x3B\xF4\x93\x2C\x70\xE1\x24\x50\x34\xC3\x54\x67\xEF\x2E\xFD\x4D\x64\xEB\xF8\x19\x68\x34\x67\xE2\xBF".to_vec() );
    let signer = Rc::new( Ed25519Signer::new(&secret_key, &public_key)
        .expect("Failed to initialize server identity"));

    let validator = Rc::new( CompositeValidator::default() );

    // TODO use persistent storages both for local and distributed
    //let distributed_storage = Box::new( Ipfs::new( "localhost", 5001, &handle1.clone() )? )
    let distributed_storage = Rc::new( RefCell::new( InMemoryStore::new() ) );
    let local_storage = Rc::new( RefCell::new( InMemoryStore::new() ) );

    let server = Rc::new( HomeServer::new(&handle, validator, distributed_storage, local_storage) );

    use std::net::ToSocketAddrs;
    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind socket");

    info!("Server started, waiting for clients");
    let done = socket.incoming().for_each( move |(socket, _addr)|
    {
        info!("Accepted client connection, serving requests");

        let handle_clone = handle.clone();
        let server_clone = server.clone();

        // TODO fill this in properly for each connection based on TLS authentication info
        let handshake_fut = handshake::temp_tcp_handshake_until_tls_is_implemented( socket, signer.clone() )
            .map_err( |e|
            {
                warn!("Client handshake failed: {:?}", e);
                ()
            } )
            .and_then( move |(reader, writer, client_context)|
            {
                let home = HomeConnectionServer::new( Rc::new(client_context), server_clone.clone() )
                    .map_err( |e| {
                        warn!("Failed to create server instance: {:?}", e);
                        ()
                    } )?;
                protocol_capnp::HomeDispatcherCapnProto::dispatch( Rc::new(home), reader, writer, handle_clone.clone() );
                Ok( () )
            } );

        handle.spawn(handshake_fut);
        Ok( () )
    } );

    let res = core.run(done);
    debug!("Reactor finished with result: {:?}", res);
    info!("Server shutdown");
}
