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

fn main() -> Fallible<()> {
    // TODO make log config path configurable or at least this should not fail if file is not available
    log4rs::init_file("log4rs.yml", Default::default())?;

    let command = Command::from_args();
    debug!("Got command {:?}", command);

    let cfg_dir = dirs::config_dir()
        .ok_or_else(|| err_msg("Failed to detect platform-dependent directory for app config"))?;
    let app_cfg_dir = Path::new(&cfg_dir).join("prometheus");
    let vault_file = "vault.dat";
    let vault_path = app_cfg_dir.join(vault_file);
    debug!("Looking for profile vault in {:?}", vault_path);

    let vault_exists = vault_path.exists();
    if command.needs_vault() && !vault_exists {
        VaultCommand::generate();
        bail!("You have to initialize vault before running {:?}", command);
    }

    let mut vault: Option<Box<ProfileVault>> = None;
    if vault_exists {
        info!("Found profile vault, loading {:?}", vault_path.to_str());
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

    let mut ctx = CommandContext::new(vault, Box::new(store));
    command.execute(&mut ctx)?;

    let vault_opt = ctx.take_vault();
    if let Some(vault) = vault_opt {
        vault.save(&app_cfg_dir, &vault_file)?;
    }
    Ok(())
}
