use failure::Fallible;
use log::*;
use std::net::SocketAddr;
use std::path::Path;
use structopt::StructOpt;

mod config;
mod state;

use state::State;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prometheus-tester",
    about = "A simulator for populating the Morpheus open social graph",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct Options {
    #[structopt(
        long = "storage",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""ADDRESS""#)
    )]
    /// IPv4/6 address of the storage backend used for this demo
    pub storage_address: SocketAddr,

    #[structopt(long = "timeout", default_value = "10", raw(value_name = r#""SECS""#))]
    /// Number of seconds used for network timeouts
    pub network_timeout_secs: u64,

    /// Number of steps to take in the simulation after resynchronization
    /// of local state with the storage backend
    #[structopt(
        long = "actions",
        default_value = "100000",
        raw(value_name = r#""STEPS""#)
    )]
    pub actions: u64,

    #[structopt(long = "state", default_value = "state.json")]
    pub state_file: String,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let options = Options::from_args();

    let config: log4rs::config::Config =
        log4rs::load_config_file("log4rs.yml", Default::default())?;
    let _log_handle = log4rs::init_config(config)?;

    debug!("Actions to take: {}", options.actions);

    let state_path = Path::new(&options.state_file);
    let mut state = if state_path.exists() {
        let state_file = std::fs::File::open(state_path)?;
        serde_json::from_reader(&state_file)?
    } else {
        State::new()
    };

    for (i, user) in state.into_iter().enumerate() {
        info!("{}: {:?}", i, user);
    }
    let idx = state.add_user();
    let user = &mut state[idx];
    user.add_link(idx);

    std::fs::create_dir_all(state_path.parent().unwrap())?;
    let cfg_file = std::fs::File::create(state_path)?;
    serde_json::to_writer_pretty(&cfg_file, &state)?;

    Ok(())
}
