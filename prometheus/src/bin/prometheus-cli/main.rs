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
    // TODO make log config path configurable or at least this should not fail if file is not available
    log4rs::init_file("log4rs.yml", Default::default())?;

    let cfg_dir = dirs::config_dir()
        .ok_or_else(|| err_msg("Failed to detect platform-dependent directory for app config"))?;
    let app_cfg_dir = Path::new(&cfg_dir).join("prometheus");
    let config_file = "vault.dat";
    let config_path = app_cfg_dir.join(config_file);
    info!("Looking for app config in {:?}", config_path);

    let vault = if config_path.exists() {
        info!("Found config, loading");
        let vault = DummyProfileVault::load(&app_cfg_dir, &config_file)?;
        vault
    } else {
        info!("No config found, generating it");
        let vault = DummyProfileVault::new();
        vault.save(&app_cfg_dir, &config_file)?;
        vault
    };
    let vault = Box::new(vault);

    let command = Command::from_args();
    info!("Got command {:?}", command);

    // TODO make address and timeout configurable
    let addr = "127.0.0.1:6161".parse()?;
    let timeout = Duration::from_secs(5);
    let store = DummyProfileStore::new(&addr, timeout)?;

    let mut ctx = CommandContext::new(vault, Box::new(store));
    command.execute(&mut ctx)?;

    ctx.vault().save(&app_cfg_dir, &config_file)?;
    Ok(())
}
