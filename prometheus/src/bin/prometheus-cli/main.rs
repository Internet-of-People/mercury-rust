use std::path::Path;
use std::time::Duration;

use dirs;
use failure::{bail, err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use crate::cli::*;
use morpheus_storage::DummyProfileStore;
use prometheus::vault::*;

mod cli;

fn main() {
    match run() {
        Ok(()) => {} // println!("OK"),
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let command = Command::from_args();

    // TODO make log config path configurable or at least this should not fail if file is not available
    log4rs::init_file("log4rs.yml", Default::default())?;

    debug!("Got command {:?}", command);

    let cfg_dir = dirs::config_dir()
        .ok_or_else(|| err_msg("Failed to detect platform-dependent directory for app config"))?;
    let app_cfg_dir = Path::new(&cfg_dir).join("prometheus");
    let vault_file = "vault.dat";
    let vault_path = app_cfg_dir.join(vault_file);

    let vault_exists = vault_path.exists();
    if command.needs_vault() && !vault_exists {
        info!(
            "Profile vault is required but not found at {}",
            vault_path.to_string_lossy()
        );
        cli::generate_vault();
        bail!(
            "First you need a profile vault initialized to run {:?}",
            command
        );
    }

    let mut vault: Option<Box<ProfileVault>> = None;
    if vault_exists {
        info!(
            "Found profile vault, loading {}",
            vault_path.to_string_lossy()
        );
        vault = Some(Box::new(DummyProfileVault::load(
            &app_cfg_dir,
            &vault_file,
        )?))
    } else {
        debug!("No profile vault found");
    }

    // TODO make address and timeout configurable
    let addr = "127.0.0.1:6161".parse()?;
    let timeout = Duration::from_secs(5);
    let store = DummyProfileStore::new(&addr, timeout)?;

    let mut ctx = CommandContext::new(vault_path, vault, Box::new(store));
    command.execute(&mut ctx)?;

    let vault_opt = ctx.take_vault();
    if let Some(vault) = vault_opt {
        vault.save(&app_cfg_dir, &vault_file)?;
    }
    Ok(())
}
