mod init;
pub mod options;
pub mod publisher;
pub mod subscriber;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;

use failure::{err_msg, format_err, Fallible};
use futures::prelude::*;
use log::*;
use structopt::StructOpt;
use tokio_core::reactor;
use tokio_signal::unix::{Signal, SIGINT};

use keyvault::{PrivateKey as KeyVaultPrivateKey, PublicKey as KeyVaultPublicKey};
use mercury_home_protocol::*;
use options::*;
use prometheus::dapp::{dapp_session::*, websocket};
use publisher::Server;
use subscriber::Client;

pub fn signal_recv(sig: i32) -> Box<dyn Stream<Item = i32, Error = failure::Error>> {
    Box::new(
        Signal::new(sig).flatten_stream().map_err(|e| format_err!("Failed to get signal: {}", e)),
    )
}

#[derive(Clone)]
pub struct AppContext {
    dapp_service: Rc<dyn DAppSessionService>,
    dapp_profile_id: ProfileId,
    home_id: ProfileId,
    app_id: ApplicationId,
    handle: reactor::Handle,
}

impl AppContext {
    pub fn new(
        profile_privatekey_file: &PathBuf,
        node_pubkey: &PublicKey,
        node_addr: &SocketAddr,
        reactor: &mut reactor::Core,
    ) -> Fallible<Self> {
        let dapp_service = Rc::new(websocket::client::ServiceClient::new());

        let private_key_bytes = std::fs::read(profile_privatekey_file)?;
        let private_key_ed = ed25519::EdPrivateKey::from_bytes(private_key_bytes)?;
        let private_key = PrivateKey::from(private_key_ed);
        let dapp_profile_id = private_key.public_key().key_id();

        let home_id = node_pubkey.key_id();
        Ok(Self {
            dapp_service,
            dapp_profile_id,
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

fn main() -> Fallible<()> {
    let options = Options::from_args();
    log4rs::init_file(&options.logger_config, Default::default()).unwrap();

    // Creating a reactor
    let mut reactor = reactor::Core::new().unwrap();

    debug!("Parsed options, initializing application");

    let priv_key_file = match options.command {
        Command::Pubhlisher(ref cfg) => &cfg.private_key_file,
        Command::Subscriber(ref cfg) => &cfg.private_key_file,
    };

    // Constructing application context from command line args
    let appcx =
        AppContext::new(priv_key_file, &options.home_pubkey, &options.home_address, &mut reactor)?;

    // Creating application object
    let app_fut = match options.command {
        Command::Pubhlisher(cfg) => Server::new(cfg, appcx).into_future(),
        Command::Subscriber(cfg) => Client::new(cfg, appcx).into_future(),
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
