pub mod cli;
pub mod client;
pub mod client_config;
mod init_hack;
pub mod logging;
pub mod server;
pub mod server_config;
// pub mod application;
// pub mod signal_handling;

use std::cell::RefCell;
use std::collections::HashSet;
//use std::net::SocketAddr;
use std::rc::Rc;

use clap::ArgMatches;
use failure::Fail;
use futures::prelude::*;
use log::*;
use tokio_core::reactor;
use tokio_signal::unix::{Signal, SIGINT};

use cli::cli;
use client::Client;
use client_config::*;
use init_hack::init_connect_service;
use logging::start_logging;
use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::profile::MyProfile;
use mercury_connect::service::ConnectService;
use mercury_connect::*;
use mercury_home_protocol::*;
use server::Server;
use server_config::*;

pub fn signal_recv(sig: i32) -> Box<Stream<Item = i32, Error = Error>> {
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
        priv_key: &str,
        node_pubkey: &str,
        node_addr: &str,
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
    //ARGUMENT HANDLING START
    let matches = cli().get_matches();

    // Print version
    if matches.is_present(cli::CLI_VERSION) {
        println!("The Button dApp 0.1 pre-alpha");
        return Ok(());
    }

    start_logging(&matches);

    // Creating a reactor
    let mut reactor = reactor::Core::new().unwrap();

    debug!("Parsed options, initializing application");

    // Constructing application context from command line args
    let appcx = AppContext::new(
        matches.value_of(cli::CLI_PRIVATE_KEY_FILE).unwrap(),
        matches.value_of(cli::CLI_HOME_NODE_PUBLIC_KEY).unwrap(),
        matches.value_of(cli::CLI_SERVER_ADDRESS).unwrap(),
        &mut reactor,
    )?;

    // Creating application object
    let (sub_name, sub_args) = matches.subcommand();
    let app_fut = match sub_args {
        Some(args) => match sub_name {
            cli::CLI_SERVER => Server::new(ServerConfig::try_from(args)?, appcx).into_future(),
            cli::CLI_CLIENT => Client::new(ClientConfig::try_from(args)?, appcx).into_future(),
            _ => {
                error!("unknown subcommand '{}'", sub_name);
                return Err(ErrorKind::LookupFailed.into());
            }
        },
        None => {
            error!("subcommand missing");
            return Err(ErrorKind::LookupFailed.into());
        }
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
