//use std::net::SocketAddr;
use std::time::Duration;

use failure::Fallible;
use log::*;

use crate::cli::*;
use prometheus::vault::*;

mod cli;

fn main() -> Fallible<()> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    use structopt::StructOpt;
    let command = Command::from_args();
    info!("Got command {:?}", command);

    let addr = "127.0.0.1:6161".parse()?;
    let timeout = Duration::from_secs(5);
    info!("Initializing profile vault, connecting to {:?}", addr);

    let vault = DummyProfileVault::new();
    let store = DummyProfileStore::new(&vault, &addr, timeout)?;

    // let vault = FailingProfileVault{};

    let ctx = CommandContext::new(Box::new(vault), Box::new(store));
    command.execute(&ctx)
}
