//#[macro_use]
extern crate clap;
extern crate either;
extern crate futures;
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate mercury_storage;
extern crate multiaddr;
extern crate multibase;
extern crate tokio_uds;
extern crate tokio_core;
extern crate tokio_signal;



pub mod client_config;
pub mod client;
pub mod server_config;
pub mod server;
pub mod cli;
pub mod logging;
pub mod function;
pub mod application;
// pub mod signal_handling;



use std::cell::RefCell;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::rc::Rc;

use clap::ArgMatches;
use futures::prelude::*;
use multiaddr::{Multiaddr, ToMultiaddr};
use tokio_signal::unix::SIGINT;
use tokio_core::reactor;

use mercury_connect::*;
use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::service::ServiceImpl;
use mercury_home_protocol::*;
use mercury_home_protocol::crypto::Ed25519Signer;
use application::{Application, EX_OK, EX_SOFTWARE, EX_USAGE};
use cli::cli;
use client::Client;
use client_config::*;
use function::*;
use logging::start_logging;
use server::Server;
use server_config::*;



fn temporary_connect_service_instance(my_private_profilekey_file: &str,
        home_id_str: &str, home_addr_str: &str, reactor: &mut reactor::Core)
    -> Result<(Rc<ServiceImpl>, ProfileId, ProfileId), std::io::Error>
{
    use mercury_connect::service::{DummyUserInterface, ProfileGatewayFactory, SignerFactory};
    use mercury_storage::async::{KeyAdapter, KeyValueStore, fs::FileStore, imp::InMemoryStore};

    debug!("Initializing service instance");

    let home_pubkey = PublicKey(std::fs::read(home_id_str)?);
    let home_id = ProfileId::from(&home_pubkey);
    let home_addr :SocketAddr = home_addr_str.parse().map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    let home_multiaddr : Multiaddr = home_addr.to_multiaddr().expect("Failed to parse server address");
    let home_profile = Profile::new_home( home_id.clone(), home_pubkey.clone(), home_multiaddr );

    let my_private_key = PrivateKey(std::fs::read(my_private_profilekey_file)?);
    let my_signer = Rc::new( Ed25519Signer::new(&my_private_key).unwrap() ) as Rc<Signer>;
    let my_profile_id = my_signer.profile_id().to_owned();
    let my_profile = Profile::new( &my_profile_id, my_signer.public_key(),
        &ProfileFacet::Persona( PersonaFacet{homes: vec![], data: vec![]} ) );

    // TODO consider that client should be able to start up without being a DHT client,
    //      e.g. with having only a Home URL including hints to access Home
    let profile_repo = SimpleProfileRepo::from( KeyAdapter::<String,_,_>::new(
        FileStore::new("/tmp/mercury/thebutton-storage").unwrap() ) );
//    let profile_repo = SimpleProfileRepo::default();
    let repo_initialized = reactor.run( profile_repo.load(&my_profile_id) );
    if repo_initialized.is_err()
    {
        debug!("Profile repository was not initialized, populate it with required entries");
        reactor.run( profile_repo.insert(home_profile) ).unwrap();
        reactor.run( profile_repo.insert(my_profile.clone() ) ).unwrap();
    }
    else { debug!("Profile repository was initialized, continue without populating it"); }
    let profile_repo = Rc::new(profile_repo);

    let my_profiles = Rc::new( vec![ my_profile_id.clone() ].iter().cloned().collect::<HashSet<_>>() );
    let my_own_profile = OwnProfile::new(&my_profile,&[]);
    let signers = vec![ ( my_profile_id.clone(), my_signer ) ].into_iter().collect();
    let signer_factory: Rc<SignerFactory> = Rc::new(SignerFactory::new(signers) );
    let home_connector = Rc::new( SimpleTcpHomeConnector::new( reactor.handle() ) );
    let gateways = Rc::new( ProfileGatewayFactory::new(
        signer_factory, profile_repo.clone(), home_connector ) );

    let ui = Rc::new( DummyUserInterface::new( my_profiles.clone() ) );
    let mut own_profile_store = InMemoryStore::new();
    reactor.run( own_profile_store.set(my_profile_id.clone(), my_own_profile ) ).unwrap();
    let profile_store = Rc::new( RefCell::new(own_profile_store) );
    let service = Rc::new( ServiceImpl::new(ui, my_profiles, profile_store, gateways, &reactor.handle() ) );

    Ok( (service, my_profile_id, home_id) )
}



fn temporary_init_env(app_context: &AppContext)
    -> Box< Future<Item=Rc<AdminEndpoint>, Error=std::io::Error> >
{
    let appctx = app_context.clone();
    let init_fut = appctx.service.admin_endpoint(None)
        .inspect( |_admin| debug!("Admin endpoint was connected") )
        // TODO no matter if we have already joined, we should normally go on as it had succeeded
        .and_then( move |admin| admin.join_home(&appctx.client_id, &appctx.home_id)
            .map( |_own_prof| admin ) )
        .inspect( |_| debug!("Successfully registered to home") )
        .map_err( |e| { debug!("Failed to register: {:?}", e); std::io::Error::new( std::io::ErrorKind::ConnectionRefused, format!("{}", e) ) } );
    Box::new(init_fut)
}



#[derive(Clone)]
pub struct AppContext{
    service: Rc<ServiceImpl>,
    client_id: ProfileId,
    home_id: ProfileId,
    handle: reactor::Handle,
}

impl AppContext
{
    pub fn new(priv_key: &str, node_id: &str, node_addr: &str, reactor: &mut reactor::Core)
        -> Result<Self, std::io::Error>
    {
        // TODO when we'll have a standalone service with proper IPC/RPC interface,
        //      this must be changed into a simple connect() call instead of building a service instance
        let (service, client_id, home_id) = temporary_connect_service_instance(priv_key, node_id, node_addr, reactor)?;
        Ok( Self{ service, client_id, home_id, handle: reactor.handle() } )
    }
}

#[derive(Debug)]
pub enum OnFail {
    Terminate,
    Retry,
}


fn application_code() -> i32 {
    match application_code_internal() {
        Ok(_) => EX_OK,
        Err(err) => {       
            error!("application failed: {}", err);
            match err.kind() {
                std::io::ErrorKind::InvalidInput => EX_USAGE,
                _ => EX_SOFTWARE
            }
        }
    }
}

fn application_code_internal() -> Result<(), std::io::Error>
{
    //ARGUMENT HANDLING START
    let matches = cli().get_matches();

    // Print version
    if matches.is_present(cli::CLI_VERSION){
        println!("The Button dApp 0.1 pre-alpha");
        return Ok(())
    }

    start_logging(&matches);

    // Creating a reactor
    let mut reactor = reactor::Core::new().unwrap();

    debug!("Parsed options, initializing application");

    // Constructing application context from command line args
    let appcx = AppContext::new(
        matches.value_of(cli::CLI_PRIVATE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_HOME_NODE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_SERVER_ADDRESS).unwrap(),
        &mut reactor )?;

    // Creating application object
    let (sub_name, sub_args) = matches.subcommand();
    let app_fut = match sub_args {
        Some(args)=>{
            match sub_name{
                cli::CLI_SERVER => Server::new( ServerConfig::try_from(args)?, appcx).into_future(),
                cli::CLI_CLIENT => Client::new( ClientConfig::try_from(args)?, appcx).into_future(),
                _=> return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("unknown subcommand '{}'", sub_name)))
            }
        },
        None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "subcommand missing"))
    };

    debug!("Initialized application, running");

    // SIGINT is terminating the server
    let sigint_fut = signal_recv(SIGINT).into_future()
        .map(|_| info!("received SIGINT, terminating application") )
        .map_err(|(err, _)| err);

    // reactor.run(app_fut)
    reactor.run(app_fut.select(sigint_fut).map(|(item, _)| item).map_err(|(err, _)| err))
}

fn main() {
    Application::run(application_code());
}
