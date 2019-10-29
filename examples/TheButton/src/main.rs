#![warn(rust_2018_idioms)]

mod init;
pub mod options;
pub mod publisher;
pub mod subscriber;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use failure::{bail, err_msg, format_err, Fallible};
use log::*;
use structopt::StructOpt;
use tokio_net::signal::unix::{signal, SignalKind};

use crate::options::*;
use crate::publisher::Server;
use crate::subscriber::Client;
use keyvault::{PrivateKey as KeyVaultPrivateKey, PublicKey as KeyVaultPublicKey};
use mercury_home_protocol::*;
use prometheus::dapp::{dapp_session::*, websocket};

#[derive(Clone)]
pub struct AppContext {
    dapp_service: Rc<dyn DAppSessionService>,
    dapp_profile_id: ProfileId,
    home_id: ProfileId,
    dapp_id: ApplicationId,
}

impl AppContext {
    pub async fn init(
        profile_privatekey_file: &PathBuf,
        home_pubkey: &PublicKey,
        home_addr: &SocketAddr,
    ) -> Fallible<Self> {
        let dapp_service = Rc::new(websocket::client::ServiceClient::new());

        // TODO private key must never be exposed directly, only a Signer (just like hardware wallets)
        let private_key_bytes = std::fs::read(profile_privatekey_file)?;
        let private_key_ed = ed25519::EdPrivateKey::from_bytes(private_key_bytes)?;
        let private_key = PrivateKey::from(private_key_ed);
        let dapp_profile_id = private_key.public_key().key_id();

        let home_id = home_pubkey.key_id();
        let this = Self {
            dapp_service,
            dapp_profile_id,
            home_id,
            dapp_id: ApplicationId("TheButton-dApp-Sample".into()),
        };
        init::ensure_registered_to_home(private_key, home_addr, &this).await?;
        Ok(this)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum OnFail {
    Terminate,
    Retry,
}

#[tokio::main]
async fn main() -> Fallible<()> {
    let options = Options::from_args();
    log4rs::init_file(&options.logger_config, Default::default()).unwrap();

    debug!("Parsed options, initializing application");

    let priv_key_file = match options.command {
        Command::Publisher(ref cfg) => &cfg.private_key_file,
        Command::Subscriber(ref cfg) => &cfg.private_key_file,
    };

    // Constructing application context from command line args
    let app_ctx =
        AppContext::init(priv_key_file, &options.home_pubkey, &options.home_address).await?;

    // Creating application object
    let app_fut = match options.command {
        Command::Publisher(cfg) => Server::new(cfg, app_ctx).checkin_and_notify().await?,
        Command::Subscriber(cfg) => Client::new(cfg, app_ctx).pair_and_listen().await?,
    };

    Ok(())
}
