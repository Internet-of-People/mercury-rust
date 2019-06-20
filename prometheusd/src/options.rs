use std::net::SocketAddr;
use std::path::PathBuf;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "prometheusd",
    about = "Prometheus service daemon",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
pub struct Options {
    #[structopt(
        long = "listen",
        default_value = "127.0.0.1:8080",
        raw(value_name = r#""IP:PORT""#)
    )]
    /// IPv4/6 address to listen on serving REST requests.
    pub listen_on: SocketAddr,

    #[structopt(long, raw(value_name = r#""DIR""#), parse(from_os_str))]
    /// Configuration directory to pick vault and profile info from.
    /// Default: OS-specific app_cfg_dir/prometheus
    pub config_dir: Option<PathBuf>,

    #[structopt(
        long = "repository",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""IP:PORT""#)
    )]
    /// IPv4/6 address of the remote profile repository.
    pub remote_repo_address: SocketAddr,

    #[structopt(long = "timeout", default_value = "10", raw(value_name = r#""SECS""#))]
    /// Number of seconds used for network timeouts
    pub network_timeout_secs: u64,

    #[structopt(
        long,
        default_value = "log4rs.yml",
        raw(value_name = r#""FILE""#),
        parse(from_os_str)
    )]
    /// Config file for log4rs (YAML).
    pub logger_config: PathBuf,
}
