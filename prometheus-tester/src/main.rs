mod config;
mod simul;
mod state;
mod sync;
mod vault;

use failure::Fallible;
use log::*;
use std::net::SocketAddr;
use std::time::Duration;
use structopt::StructOpt;

use osg_rpc_storage::RpcProfileRepository;

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
        default_value = "172.17.0.2:6161",
        raw(value_name = r#""ADDRESS""#)
    )]
    /// IPv4/6 address of the storage backend used for this demo
    pub storage_address: SocketAddr,

    #[structopt(long = "timeout", default_value = "10", raw(value_name = r#""SECS""#))]
    /// Number of seconds used for network timeouts
    pub network_timeout_secs: u64,

    /// Number of steps to take in the simulation after resynchronization
    /// of local state with the storage backend
    #[structopt(long = "actions", default_value = "200", raw(value_name = r#""STEPS""#))]
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

    let cwd = std::fs::canonicalize("./")?;
    let state_path = cwd.join(&options.state_file);
    debug!("State: {}", state_path.to_string_lossy());
    let mut state = if state_path.exists() {
        let state_file = std::fs::File::open(&state_path)?;
        serde_json::from_reader(&state_file)?
    } else {
        State::new("include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already").unwrap()
    };

    let timeout = Duration::from_secs(options.network_timeout_secs);
    let mut repo = RpcProfileRepository::new(&options.storage_address, timeout)?;

    info!("Synchronizing existing state");
    sync::synchronize(&mut state, &mut repo)?;
    info!("Starting simulation");
    let mut sim = simul::Simulation::new(&mut state, &mut repo)?;
    for i in 1..=options.actions {
        if i % 100 == 0 {
            print_stats(&sim)?;
        }
        sim.step()?;
    }
    info!("Finished simulation");
    print_stats(&sim)?;

    std::fs::create_dir_all(&state_path.parent().unwrap())?;
    let cfg_file = std::fs::File::create(state_path)?;
    serde_json::to_writer_pretty(&cfg_file, &state)?;

    Ok(())
}

fn print_stats(sim: &simul::Simulation) -> Fallible<()> {
    let (steps, nodes, links, influencers) = sim.stats()?;
    info!("..{} steps, {} nodes, {} links", steps, nodes, links);
    let x: std::borrow::Cow<'_, [String]> = influencers.iter().map(|i| format!("{}", i)).collect();
    info!("  Top follower counts: {}", x.join(", "));
    Ok(())
}