use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::rc::Rc;

use log::*;
use structopt::StructOpt;

use mercury_home_protocol::*;
use osg::vault::{HdProfileVault, ProfileVault};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mercury-home",
    about = "Mercury Home Node daemon",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct CliConfig {
    #[structopt(long = "keyvault-dir", raw(value_name = r#""DIR""#), parse(from_os_str))]
    /// Configuration directory to load keyvault from.
    /// Default: OS-specific app_cfg_dir/prometheus
    pub keyvault_dir: Option<PathBuf>,

    #[structopt(long = "profileid", raw(value_name = r#""ID""#))]
    /// Key ID within keyvault to be used for authentication by this node.
    pub profile_id: Option<ProfileId>,

    #[structopt(
        long = "private-storage",
        default_value = "/tmp/mercury/home/hosted-profiles",
        parse(from_os_str),
        raw(value_name = r#""PATH""#)
    )]
    /// Directory path to store hosted profiles in
    private_storage_path: PathBuf,

    #[structopt(
        long = "distributed-storage",
        default_value = "127.0.0.1:6161",
        raw(value_name = r#""IP:PORT""#)
    )]
    /// Network address of public profile storage
    distributed_storage_address: String,

    #[structopt(long = "tcp", default_value = "0.0.0.0:2077", raw(value_name = r#""IP:Port""#))]
    /// Listen on this socket to serve TCP clients
    socket_addr: String,
}

impl CliConfig {
    const CONFIG_PATH: &'static str = "home.cfg";

    pub fn new() -> Self {
        util::parse_config::<Self>(Self::CONFIG_PATH)
    }
}

pub struct Config {
    private_storage_path: PathBuf,
    distributed_storage_address: SocketAddr,
    signer: Rc<Signer>,
    listen_socket: SocketAddr, // TODO consider using Vec if listening on several network devices is needed
}

impl Config {
    pub fn new() -> Self {
        let cli = CliConfig::new();

        let vault_path =
            osg::paths::vault_path(cli.keyvault_dir).expect("Failed to get keyvault path");
        let vault = HdProfileVault::load(&vault_path).expect(&format!(
            "Profile vault is required but failed to load from {}",
            vault_path.to_string_lossy()
        ));
        let private_keys = vault.secrets().expect("failed to get list of owned keys");

        let profile_id = cli.profile_id.or_else(|| vault.get_active().expect("Failed to get active profile") )
            .expect("Profile id is needed for authenticating the node, but neither command line argument is specified, nor active profile is set in vault");
        let key_idx = vault
            .index_of(&profile_id)
            .expect(&format!("Specified id is not found in vault: {}", profile_id));
        let private_key =
            private_keys.private_key(key_idx as i32).expect("Failed to get first key");
        let signer =
            Rc::new(crypto::PrivateKeySigner::new(private_key).expect("Failed to create signer"));

        info!("homenode profile id: {}", signer.profile_id());
        info!("homenode public key: {}", signer.public_key());

        let listen_socket = cli
            .socket_addr
            .to_socket_addrs()
            .unwrap()
            .next()
            .expect("Failed to parse socket address for private storage");

        let distributed_storage_address = cli
            .distributed_storage_address
            .to_socket_addrs()
            .unwrap()
            .next()
            .expect("Failed to parse socket address for distributed storage");

        Self {
            private_storage_path: cli.private_storage_path,
            distributed_storage_address,
            signer,
            listen_socket,
        }
    }

    pub fn private_storage_path(&self) -> &PathBuf {
        &self.private_storage_path
    }
    pub fn distributed_storage_address(&self) -> &SocketAddr {
        &self.distributed_storage_address
    }
    pub fn signer(&self) -> Rc<Signer> {
        self.signer.clone()
    }
    pub fn listen_socket(&self) -> &SocketAddr {
        &self.listen_socket
    }
}
