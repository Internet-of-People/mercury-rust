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
        .ok_or_else(|| err_msg("Failed to detect platform-dependent directory for app config"))?;
    let app_cfg_dir = Path::new(&cfg_dir).join("prometheus");
    let config_path = app_cfg_dir.join("config");
    info!("Looking for app config in {:?}", config_path);

    let vault = if config_path.exists() {
        info!("Found config, loading");
        let cfg_file = std::fs::File::open(&config_path)?;
        let vault: DummyProfileVault = serde_json::from_reader(&cfg_file)?;
        vault
    } else {
        info!("No config found, generating it");
        let vault = DummyProfileVault::new();
        std::fs::create_dir_all(&app_cfg_dir)?;
        let cfg_file = std::fs::File::create(&config_path)?;
        serde_json::to_writer_pretty(&cfg_file, &vault)?;
        vault
    };

    let command = Command::from_args();
    info!("Got command {:?}", command);

    let addr = "127.0.0.1:6161".parse()?;
    let timeout = Duration::from_secs(5);
    let store = DummyProfileStore::new(&addr, timeout)?;

    let mut ctx = CommandContext::new(Box::new(vault), Box::new(store));
    command.execute(&mut ctx)
}
