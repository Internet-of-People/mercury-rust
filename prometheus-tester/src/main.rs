use failure::Fallible;
use log::*;
use std::net::SocketAddr;
use structopt::StructOpt;

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

    #[structopt(
        long = "actions",
        default_value = "100000",
        raw(value_name = r#""STEPS""#)
    )]
    pub actions: u64,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let options = Options::from_args();

    // TODO make log config path configurable or at least this should not fail if file is not available
    log4rs::init_file("log4rs.yml", Default::default())?;

    debug!("Actions to take: {}", options.actions);
    Ok(())
}
