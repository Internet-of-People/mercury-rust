use failure::Fallible;
use log::*;
use std::time::Duration;

use crate::cli::*;
use prometheus::store::*;
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

    let mut vault = DummyProfileVault::new();
    let store = DummyProfileStore::new(&mut vault, &addr, timeout)?;

    // let vault = FailingProfileVault{};

    let mut ctx = CommandContext::new(Box::new(vault), Box::new(store));
    command.execute(&mut ctx)
}
