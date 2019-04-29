use std::cell::RefCell;
use std::rc::Rc;

use futures::{Future, Stream};
use log::*;
use tokio_core::{net::TcpListener, reactor};

use mercury_home_node::{config::*, server::*};
use mercury_home_protocol::{
    crypto::*, handshake, mercury_capnp::server_dispatcher::HomeDispatcherCapnProto,
};
use mercury_storage::asynch::imp::InMemoryStore;
use osg::repo::FileProfileRepository;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let config = Config::new();

    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    // TODO use some kind of persistent storage for public distributed storage
    //let distributed_storage = Box::new( Ipfs::new( "localhost", 5001, &handle1.clone() )? )
    let distributed_storage = Rc::new(RefCell::new(InMemoryStore::new()));
    let local_storage =
        Rc::new(RefCell::new(FileProfileRepository::new(config.storage_path()).unwrap()));
    let signer = config.signer();
    let validator = Rc::new(CompositeValidator::default());
    let server = Rc::new(HomeServer::new(&handle, validator, distributed_storage, local_storage));

    info!("Opening socket {} for incoming TCP clients", config.listen_socket());
    let socket = TcpListener::bind(config.listen_socket(), &handle).expect("Failed to bind socket");

    info!("Server started, waiting for clients");
    let done = socket.incoming().for_each(move |(socket, _addr)| {
        info!("Accepted client connection, serving requests");

        let handle_clone = handle.clone();
        let server_clone = server.clone();

        // TODO fill this in properly for each connection based on TLS authentication info
        let handshake_fut = handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done(
            socket,
            signer.clone(),
        )
        .map_err(|e| warn!("Client handshake failed: {:?}", e))
        .and_then(move |(reader, writer, client_context)| {
            let home = HomeConnectionServer::new(Rc::new(client_context), server_clone.clone())
                .map_err(|e| warn!("Failed to create server instance: {:?}", e))?;
            HomeDispatcherCapnProto::dispatch(Rc::new(home), reader, writer, handle_clone.clone());
            Ok(())
        });

        handle.spawn(handshake_fut);
        Ok(())
    });

    let res = core.run(done);
    debug!("Reactor finished with result: {:?}", res);
    info!("Server shutdown");
}
