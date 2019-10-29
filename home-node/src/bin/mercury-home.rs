use std::cell::RefCell;
use std::rc::Rc;

use failure::format_err;
use log::*;
use tokio::net::tcp::TcpListener;
use tokio::prelude::*;

use claims::repo::{DistributedPublicProfileRepository, FileProfileRepository};
use mercury_home_node::{config::*, server::*};
use mercury_home_protocol::{
    crypto::*, handshake, mercury_capnp::server_dispatcher::HomeDispatcherCapnProto, *,
};
use mercury_storage::asynch::fs::FileStore;
use mercury_storage::asynch::KeyAdapter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log4rs::init_file("log4rs.yml", Default::default())?;
    let config = Config::new();

    let signer = config.signer();
    let validator = Rc::new(CompositeValidator::default());

    let local_storage =
        Rc::new(RefCell::new(FileProfileRepository::new(config.profile_backup_path())?));

    // TODO make file path configurable, remove rpc_storage address config parameter
    // TODO use some kind of real distributed storage here on the long run
    let mut distributed_storage =
        FileProfileRepository::new(&std::path::PathBuf::from("/tmp/cuccos"))?;
    let avail_prof_res = distributed_storage.get_public(&signer.profile_id()).await;
    if avail_prof_res.is_err() {
        info!("Home node profile is not found on distributed public storage, saving node profile");
        use multiaddr::ToMultiaddr;
        let home_multiaddr = config.listen_socket().to_multiaddr()?;
        let home_attrs = HomeFacet::new(vec![home_multiaddr], vec![]).to_attribute_map();
        let home_profile = Profile::new(signer.public_key(), 1, vec![], home_attrs);
        distributed_storage.set_public(home_profile).await?;
    } else {
        info!("Home node profile is already available on distributed public storage");
    }

    let host_db =
        Rc::new(RefCell::new(KeyAdapter::new(FileStore::new(config.host_relations_path())?)));
    let distributed_storage = Rc::new(RefCell::new(distributed_storage));
    let server = Rc::new(HomeServer::new(validator, distributed_storage, local_storage, host_db));

    info!("Opening socket {} for incoming TCP clients", config.listen_socket());
    let bound_socket = TcpListener::bind(config.listen_socket())
        .await
        .map_err(|e| format_err!("Failed to bind socket: {}", e))?;

    info!("Server started, waiting for clients");
    let mut accept_stream = bound_socket.incoming();
    while let Some(socket_res) = accept_stream.next().await {
        match socket_res {
            Err(e) => warn!("Failed to accept socket: {}", e),
            Ok(socket) => {
                info!("Accepted client connection, serving requests");

                let server_clone = server.clone();

                // TODO fill this in properly for each connection based on TLS authentication info
                let handshake_res =
                    handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done(
                        socket,
                        signer.clone(),
                    )
                    .await;
                match handshake_res {
                    Err(e) => warn!("Client handshake failed: {:?}", e),
                    Ok((client_context, reader, writer)) => {
                        match HomeConnectionServer::new(
                            Rc::new(client_context),
                            server_clone.clone(),
                        ) {
                            Err(e) => warn!("Failed to create server instance: {:?}", e),
                            Ok(home) => {
                                HomeDispatcherCapnProto::dispatch(Rc::new(home), reader, writer)
                            }
                        };
                    }
                };
            }
        };
    }
    info!("Server shutdown");
    Ok(())
}
