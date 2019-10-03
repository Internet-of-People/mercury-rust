pub mod client;
mod init_hack;
pub mod options;
pub mod server;

use std::cell::RefCell;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;

use failure::Fail;
use futures::prelude::*;
use log::*;
use structopt::StructOpt;
use tokio_core::reactor;
use tokio_signal::unix::{Signal, SIGINT};

use client::Client;
use init_hack::init_connect_service;
use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::profile::MyProfile;
use mercury_connect::service::ConnectService;
use mercury_connect::*;
use mercury_home_protocol::*;
use options::*;
use server::Server;

pub fn signal_recv(sig: i32) -> Box<dyn Stream<Item = i32, Error = Error>> {
    Box::new(
        Signal::new(sig)
            .flatten_stream()
            .map_err(|e| e.context(ErrorKind::ImplementationError).into()),
    )
}

#[derive(Clone)]
pub struct AppContext {
    service: Rc<ConnectService>,
    client_id: ProfileId,
    home_id: ProfileId,
    app_id: ApplicationId,
    handle: reactor::Handle,
}

impl AppContext {
    pub fn new(
        priv_key: &PathBuf,
        node_pubkey: &PublicKey,
        node_addr: &SocketAddr,
        reactor: &mut reactor::Core,
    ) -> Result<Self, Error> {
        // TODO when we'll have a standalone service with proper IPC/RPC interface,
        //      this must be changed into a simple connect() call instead of building a service instance
        let (service, client_id, home_id) =
            init_connect_service(priv_key, node_pubkey, node_addr, reactor)?;
        Ok(Self {
            service,
            client_id,
            home_id,
            handle: reactor.handle(),
            app_id: ApplicationId("TheButton-dApp-Sample".into()),
        })
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum OnFail {
    Terminate,
    Retry,
}

fn main() -> Result<(), Error> {
    let options = Options::from_args();
    log4rs::init_file(&options.logger_config, Default::default()).unwrap();

    // Creating a reactor
    let mut reactor = reactor::Core::new().unwrap();

    debug!("Parsed options, initializing application");

    let priv_key_file = match options.command {
        Command::Server(ref cfg) => &cfg.private_key_file,
        Command::Client(ref cfg) => &cfg.private_key_file,
    };

    // Constructing application context from command line args
    let appcx =
        AppContext::new(priv_key_file, &options.home_pubkey, &options.home_address, &mut reactor)?;

    // Creating application object
    let app_fut = match options.command {
        Command::Server(cfg) => Server::new(cfg, appcx).into_future(),
        Command::Client(cfg) => Client::new(cfg, appcx).into_future(),
    };

    debug!("Initialized application, running");

    // SIGINT is terminating the server
    let sigint_fut = signal_recv(SIGINT)
        .into_future()
        .map(|_| info!("received SIGINT, terminating application"))
        .map_err(|(err, _)| err);

    // reactor.run(app_fut)
    reactor.run(app_fut.select(sigint_fut).map(|(item, _)| item).map_err(|(err, _)| err))
}
