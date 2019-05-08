use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::rc::Rc;

use log::*;
use structopt::StructOpt;

use mercury_home_protocol::*;
use osg::vault::HdProfileVault;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mercury-home",
    about = "Mercury Home Node daemon",
    raw(setting = "structopt::clap::AppSettings::ColoredHelp")
)]
struct CliConfig {
    #[structopt(long = "keyvault-dir", raw(value_name = r#""DIR""#), parse(from_os_str))]
    /// Configuration directory to pick vault from.
    /// Default: OS-specific app_cfg_dir/prometheus
    pub keyvault_dir: Option<PathBuf>,

    #[structopt(
        long = "private-storage",
        default_value = "/tmp/mercury/home/hosted-profiles",
        parse(from_os_str),
        help = "Directory path to store hosted profiles in",
        raw(value_name = r#""path/to/dir""#)
    )]
    private_storage_path: PathBuf,

    #[structopt(
        long = "distributed-storage",
        default_value = "127.0.0.1:6161",
        help = "Network address of public profile storage",
        raw(value_name = r#""IP:PORT""#)
    )]
    distributed_storage_address: String,

    #[structopt(
        long = "tcp",
        default_value = "0.0.0.0:2077",
        raw(value_name = r#""IP:Port""#),
        help = "Listen on this socket to serve TCP clients"
    )]
    socket_addr: String,
}

impl CliConfig {
    const CONFIG_PATH: &'static str = "home.cfg";

    pub fn new() -> Self {
        util::parse_config::<Self>(Self::CONFIG_PATH)
    }
}

pub struct Config {
    //storage_path: String,
    private_storage_path: PathBuf,
    distributed_storage_address: SocketAddr,
    signer: Rc<Signer>,
    listen_socket: SocketAddr, // TODO consider using Vec if listening on several network devices is needed
}

impl Config {
    pub fn new() -> Self {
        let cli = CliConfig::new();

        // NOTE for some test keys see https://github.com/tendermint/signatory/blob/master/src/ed25519/test_vectors.rs
        //let bytes = fs::read(cli.private_key_file).unwrap();
        //let edpk = ed25519::EdPrivateKey::from_bytes(bytes).unwrap();
        //let signer = Rc::new(
        //    crypto::PrivateKeySigner::new(PrivateKey::from(edpk)).expect("Invalid private key"),
        //);

        let vault_path =
            osg::paths::vault_path(cli.keyvault_dir).expect("Failed to get keyvault path");
        let keyvault = HdProfileVault::load(&vault_path).expect("Failed to load profile vault");
        let private_keys = keyvault.secrets().expect("failed to get list of owned keys");
        // TODO make the key selectable (explicit option or active keyvault default) instead of hardwired first one
        let private_key = private_keys.private_key(0).expect("Failed to get first key");
        let signer =
            Rc::new(crypto::PrivateKeySigner::new(private_key).expect("Failed to create signer"));

        info!("homenode public key: {}", signer.public_key());
        info!("homenode profile id: {}", signer.profile_id());

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

        // Self { storage_path, signer, listen_socket }
        Self {
            private_storage_path: cli.private_storage_path,
            distributed_storage_address,
            signer,
            listen_socket,
        }
    }

    // pub fn storage_path(&self) -> &str {
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
