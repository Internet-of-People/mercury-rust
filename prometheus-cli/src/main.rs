use std::time::Duration;

use failure::{bail, Fallible};
use log::*;
use structopt::StructOpt;

use crate::options::{Command, Options};
use did::api::*;
use did::repo::*;
use did::vault::*;
use osg_rpc_storage::RpcProfileRepository;

mod options;
mod seed;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let options = Options::from_args();
    init_logger(&options)?;

    let command = options.command;
    debug!("Got command {:?}", command);

    let vault_path = did::paths::vault_path(options.config_dir.clone())?;
    let repo_path = did::paths::profile_repo_path(options.config_dir.clone())?;
    let base_path = did::paths::base_repo_path(options.config_dir.clone())?;

    let vault_exists = vault_path.exists();
    if command.needs_vault() && !vault_exists {
        error!("Profile vault is required but not found at {}", vault_path.to_string_lossy());
        seed::show_generated_phrase();
        bail!("First you need a profile vault initialized to run {:?}", command);
    }

    let mut vault: Option<Box<ProfileVault>> = None;
    if vault_exists {
        info!("Found profile vault, loading {}", vault_path.to_string_lossy());
        vault = Some(Box::new(HdProfileVault::load(&vault_path)?))
    } else {
        debug!("No profile vault found");
    }

    let local_repo = FileProfileRepository::new(&repo_path)?;
    let base_repo = FileProfileRepository::new(&base_path)?;
    let timeout = Duration::from_secs(options.network_timeout_secs);
    let rpc_repo = RpcProfileRepository::new(&options.remote_repo_address, timeout)?;

    let mut ctx = Context::new(
        vault_path.clone(),
        vault,
        local_repo,
        Box::new(base_repo),
        Box::new(rpc_repo.clone()),
        Box::new(rpc_repo),
    );
    let command = Box::new(command);
    command.execute(&mut ctx)?;

    let vault_opt = ctx.take_vault();
    if let Some(vault) = vault_opt {
        vault.save(&vault_path)?;
    }
    Ok(())
}

fn init_logger(options: &Options) -> Fallible<()> {
    if log4rs::init_file(&options.logger_config, Default::default()).is_err() {
        println!(
            "Failed to initialize loggers from config file {:?}, use default config",
            options.logger_config
        );

        use log4rs::append::console::ConsoleAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let stdout =
            ConsoleAppender::builder().encoder(Box::new(PatternEncoder::new("{m}{n}"))).build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    };
    Ok(())
}
