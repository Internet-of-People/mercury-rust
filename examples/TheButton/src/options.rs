use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::StructOpt;

use did::model::*;

#[derive(Debug, StructOpt)]
#[structopt(
    setting = structopt::clap::AppSettings::ColoredHelp
)]
pub struct Options {
    #[structopt(long = "repository", default_value = "127.0.0.1:6161", value_name = "IP:PORT")]
    /// IPv4/6 address of the remote profile repository. Temporary solution until proper routing is in place.
    pub profile_repo_address: SocketAddr,

    #[structopt(long, default_value = "127.0.0.1:2077", value_name = "IP:PORT")]
    /// IPv4/6 address to listen on serving REST requests.
    pub home_address: SocketAddr,

    #[structopt(long, value_name = "KEY")]
    /// Public key (multicipher) of the Home node to host this dApp
    pub home_pubkey: PublicKey,

    #[structopt(long, default_value = "log4rs.yml", value_name = "FILE", parse(from_os_str))]
    /// Config file for log4rs (YAML).
    pub logger_config: PathBuf,

    #[structopt(subcommand)]
    pub command: Command,
    //    #[structopt(long, value_name = "DIR", parse(from_os_str))]
    //    /// Configuration directory to pick vault and profile info from.
    //    /// Default: OS-specific app_cfg_dir/prometheus
    //    pub config_dir: Option<PathBuf>,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    #[structopt(name = "publisher")]
    /// Generate a phraselist needed to create a profile vault
    Pubhlisher(PublisherConfig),

    #[structopt(name = "subscriber")]
    /// Restore profile vault from a phraselist or profile from remote repository
    Subscriber(SubscriberConfig),
}

#[derive(Clone, Debug, StructOpt)]
pub struct PublisherConfig {
    #[structopt(
        long,
        default_value = "../../etc/server.id",
        value_name = "FILE",
        parse(from_os_str)
    )]
    /// File to load ed25519 server private key from. Temporary solution until keyvault is used here.
    pub private_key_file: PathBuf,

    #[structopt(long, value_name = "SECS")]
    /// Automatically push button periodically with this given time interval
    pub event_timer_secs: Option<u64>,
}

#[derive(Clone, Debug, StructOpt)]
pub struct SubscriberConfig {
    #[structopt(
        long,
        default_value = "../../etc/client.id",
        value_name = "FILE",
        parse(from_os_str)
    )]
    /// File to load ed25519 client private key from. Temporary solution until keyvault is used here.
    pub private_key_file: PathBuf,

    #[structopt(long, value_name = "DID")]
    /// Profile Id of the dApp server side to contact
    pub server_id: ProfileId,
}
