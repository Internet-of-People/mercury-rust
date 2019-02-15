use std::path::Path;
use std::time::Duration;

use dirs;
use failure::{err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use crate::cli::*;
use morpheus_storage::DummyProfileStore;
use prometheus::vault::*;

mod cli;

fn main() -> Fallible<()> {
    log4rs::init_file("log4rs.yml", Default::default())?;

    let cfg_dir = dirs::config_dir()
        .ok_or_else( || err_msg("Failed to detect platform-dependent directory for app config") )?;
    let config_path = Path::new(&cfg_dir).join("prometheus").join("config");
    info!("Looking for app state in {:?}", config_path);
    let config_state = if config_path.exists() {
        //load()
    }
    else {

    };

    let command = Command::from_args();
    info!("Got command {:?}", command);

    let addr = "127.0.0.1:6161".parse()?;
    let timeout = Duration::from_secs(5);

    let vault = DummyProfileVault::new();
    let store = DummyProfileStore::new(&addr, timeout)?;

    let mut ctx = CommandContext::new(Box::new(vault), Box::new(store));
    command.execute(&mut ctx)
}
