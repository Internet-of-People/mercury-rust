use std::time::Duration;

use dirs;
use failure::{bail, err_msg, Fallible};
use log::*;
use structopt::StructOpt;

use crate::cli::*;
use osg::vault::*;
use osg_rpc_storage::RpcProfileRepository;

mod cli;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let options = Options::from_args();
    let command = options.command;

    if log4rs::init_file(&options.logger_config, Default::default()).is_err() {
        println!(
            "Failed to initialize loggers from config file {:?}, falling back to default loggers",
            options.logger_config
        );

        use log::LevelFilter;
        use log4rs::append::console::ConsoleAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let stdout = ConsoleAppender::builder()
            .encoder(Box::new(PatternEncoder::new("{m}{n}")))
            .build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    }

    debug!("Got command {:?}", command);

    let app_cfg_dir = dirs::config_dir()
        .ok_or_else(|| err_msg("Failed to detect platform-dependent directory for app config"))?;
    let prometheus_cfg_dir = app_cfg_dir.join("prometheus");

    let vault_path = options
        .keyvault_path
        .unwrap_or_else(|| prometheus_cfg_dir.join("vault.dat"));
    let _repo_path = options
        .profile_repo_path
        .unwrap_or_else(|| prometheus_cfg_dir.join("profiles.dat"));

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
        vault = Some(Box::new(HdProfileVault::load(&vault_path)?))
    } else {
        debug!("No profile vault found");
    }

    let timeout = Duration::from_secs(options.network_timeout_secs);
    let repository = RpcProfileRepository::new(&options.storage_address, timeout)?;

    let mut ctx = CommandContext::new(vault_path.clone(), vault, Box::new(repository));
    command.execute(&mut ctx)?;

    let vault_opt = ctx.take_vault();
    if let Some(vault) = vault_opt {
        vault.save(&vault_path, Some(&app_cfg_dir))?;
    }
    Ok(())
}
