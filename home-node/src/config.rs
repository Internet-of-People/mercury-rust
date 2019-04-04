use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;
use std::rc::Rc;

use log::*;
use structopt::StructOpt;

use mercury_home_protocol::{*, crypto::*};




#[derive(Debug, StructOpt)]
struct CliConfig
{
    #[structopt(long="server-key", default_value="../etc/homenode.id", parse(from_os_str), raw(value_name=r#""FILE""#),
        help="Private key file used to prove server identity. Currently only ed25519 keys are supported in raw binary format")]
    private_key_file: PathBuf,

    // TODO default value is only for testing, make this platform-dependent
    #[structopt(long="storage", default_value="/tmp/mercury/home/hosted-profiles", parse(from_os_str),
        help="Directory path to store hosted profiles in", raw(value_name=r#""path/to/dir""#) )]
    storage_path: PathBuf,

    #[structopt(long="tcp", default_value="0.0.0.0:2077", raw(value_name=r#""IP:Port""#),
        help="Listen on this socket to serve TCP clients")]
    socket_addr: String,
}

impl CliConfig
{
    const CONFIG_PATH: &'static str = "home.cfg";

    pub fn new() -> Self
        { util::parse_config::<Self>(Self::CONFIG_PATH) }
}



pub struct Config
{
    storage_path: String,
    signer: Rc<Signer>,
    listen_socket: SocketAddr, // TODO consider using Vec if listening on several network devices is needed
}

impl Config
{
    pub fn new() -> Self
    {
        let cli = CliConfig::new();

        let storage_path = cli.storage_path.to_str()
            .expect("Storage path should have a default value").to_owned();

        // TODO support hardware wallets
        // TODO consider supporting base64 and/or multibase parsing
        // NOTE for some test keys see https://github.com/tendermint/signatory/blob/master/src/ed25519/test_vectors.rs
        let private_key = PrivateKey( fs::read(cli.private_key_file).unwrap() );
        let signer = Rc::new( Ed25519Signer::new(&private_key)
            .expect("Invalid private key") );

        info!("homenode public key: {}", signer.public_key());
        info!("homenode profile id: {}", signer.profile_id());

        let listen_socket = cli.socket_addr
            .to_socket_addrs().unwrap().next().expect("Failed to parse socket address");

        Self{storage_path, signer, listen_socket}
    }

    pub fn storage_path(&self) -> &str { &self.storage_path }
    pub fn signer(&self) -> Rc<Signer> { self.signer.clone() }
    pub fn listen_socket(&self) -> &SocketAddr { &self.listen_socket }
}
