use std::cell::RefCell;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::rc::Rc;

use failure::Fail;
//use futures::prelude::*;
use log::*;
use multiaddr::ToMultiaddr;
use structopt::StructOpt;
use tokio_codec::LinesCodec;
use tokio_core::reactor;

use mercury_connect::jsonrpc;
use mercury_connect::service::*;
use mercury_connect::*;
use mercury_home_protocol::crypto::*;
use mercury_home_protocol::keyvault::PublicKey as KeyVaultPublicKey;
use mercury_home_protocol::*;

pub fn init_connect_service(
    my_profile_privkey_file: &PathBuf,
    home_pubkey_file: &PathBuf,
    home_addr_str: &str,
    reactor: &mut reactor::Core,
) -> Result<(Rc<ConnectService>, ProfileId, ProfileId), Error> {
    use claims::repo::FileProfileRepository;

    debug!("Initializing service instance");

    let home_pubkey_bytes = std::fs::read(home_pubkey_file)
        .map_err(|e| Error::from(e.context(ErrorKind::LookupFailed)))?;
    let home_pubkey_ed = ed25519::EdPublicKey::from_bytes(home_pubkey_bytes)
        .map_err(|e| Error::from(e.context(ErrorKind::LookupFailed)))?;
    let home_pubkey = PublicKey::from(home_pubkey_ed);
    let home_id = home_pubkey.key_id();
    let home_addr: SocketAddr =
        home_addr_str.parse().map_err(|_e| Error::from(ErrorKind::LookupFailed))?;
    let home_multiaddr = home_addr.to_multiaddr().expect("Failed to parse server address");
    let home_attrs = HomeFacet::new(vec![home_multiaddr], vec![]).to_attributes();
    let home_profile = Profile::new(home_pubkey, 1, vec![], home_attrs);

    let my_private_key_bytes = std::fs::read(my_profile_privkey_file)
        .map_err(|e| Error::from(e.context(ErrorKind::LookupFailed)))?;
    let my_private_key_ed = ed25519::EdPrivateKey::from_bytes(my_private_key_bytes)
        .map_err(|e| Error::from(e.context(ErrorKind::LookupFailed)))?;
    let my_private_key = PrivateKey::from(my_private_key_ed);
    let my_signer = Rc::new(PrivateKeySigner::new(my_private_key).unwrap()) as Rc<Signer>;
    let my_profile_id = my_signer.profile_id().to_owned();
    let my_attrs = PersonaFacet::new(vec![], vec![]).to_attributes();
    let my_profile = Profile::new(my_signer.public_key(), 1, vec![], my_attrs);

    // TODO consider that client should be able to start up without being a DHT client,
    //      e.g. with having only a Home URL including hints to access Home
    let mut profile_repo =
        FileProfileRepository::new(&PathBuf::from("/tmp/mercury/connect/profile-repo")).unwrap();
    let repo_initialized = reactor.run(profile_repo.get_public(&my_profile_id));
    if repo_initialized.is_err() {
        debug!("Profile repository was not initialized, populate it with required entries");
        reactor.run(profile_repo.set_public(home_profile)).unwrap();
        reactor.run(profile_repo.set_public(my_profile.clone())).unwrap();
    } else {
        debug!("Profile repository was initialized, continue without populating it");
    }

    let my_profiles = Rc::new(vec![my_profile_id.clone()].iter().cloned().collect::<HashSet<_>>());
    let my_own_profile = OwnProfile::from_public(my_profile);
    let signers = vec![(my_profile_id.clone(), my_signer)].into_iter().collect();
    let signer_factory: Rc<SignerFactory> = Rc::new(SignerFactory::new(signers));
    let home_connector = Rc::new(SimpleTcpHomeConnector::new(reactor.handle()));
    let profile_client_factory = Rc::new(MyProfileFactory::new(
        signer_factory,
        Rc::new(RefCell::new(profile_repo)),
        home_connector,
        reactor.handle(),
    ));

    let ui = Rc::new(DummyUserInterface::new(my_profiles.clone()));
    let mut own_profile_store =
        FileProfileRepository::new(&PathBuf::from("/tmp/mercury/connect/my-profiles")).unwrap();
    reactor.run(own_profile_store.set(my_own_profile)).unwrap();
    let profile_store = Rc::new(RefCell::new(own_profile_store));
    let service =
        Rc::new(ConnectService::new(ui, my_profiles, profile_store, profile_client_factory)); //, &reactor.handle() ) );

    Ok((service, my_profile_id, home_id))
}

#[derive(Debug, StructOpt)]
struct Config {
    #[structopt(
        long = "my-private-key",
        default_value = "../etc/client.id",
        raw(value_name = r#""FILE""#),
        parse(from_os_str),
        help = "Private key file used to prove server identity. Currently only ed25519 keys are supported in raw binary format"
    )]
    my_private_key_file: PathBuf,

    #[structopt(
        long = "home-public-key",
        default_value = "../etc/homenode.id.pub",
        raw(value_name = r#""FILE""#),
        parse(from_os_str),
        help = "Public key file of home node used by the selected profile"
    )]
    home_public_key_file: PathBuf,

    #[structopt(
        long = "home-address",
        default_value = "127.0.0.1:2077",
        raw(value_name = r#""IP:PORT""#),
        help = "TCP address of the home node to be connected"
    )]
    home_address: String,

    //    #[structopt(long="jsonrpc-tcp", default_value="0.0.0.0:2222", raw(value_name=r#""IP:PORT""#),
    //        help="Listen on this socket to serve JsonRpc clients via TCP")]
    //    tcp_address: String,
    #[structopt(
        long = "jsonrpc-uds",
        default_value = "/tmp/jsonrpc.sock",
        raw(value_name = r#""FILE""#),
        parse(from_os_str),
        help = "Socket file path to serve JsonRpc clients via Unix Domain Sockets"
    )]
    uds_path: PathBuf,
}

impl Config {
    const CONFIG_PATH: &'static str = "connect.cfg";

    pub fn new() -> Self {
        util::parse_config(Self::CONFIG_PATH)
    }
}

fn main() -> Result<(), Error> {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    let config = Config::new();
    println!("Config: {:?}", config);

    let mut reactor = reactor::Core::new().unwrap();
    let (service, _my_profile_id, _home_id) = init_connect_service(
        &config.my_private_key_file,
        &config.home_public_key_file,
        &config.home_address,
        &mut reactor,
    )?;

    let jsonrpc = jsonrpc::UdsServer::new(&config.uds_path, reactor.handle()).unwrap();
    let jsonrpc_fut = jsonrpc.dispatch(LinesCodec::new(), service);
    reactor.run(jsonrpc_fut)

    // TODO react to ctrl-c with proper cleanup (running drops) instead of abort
}
