//#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;

extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate tokio_uds;
extern crate tokio_core;
extern crate tokio_timer;
extern crate tokio_signal;
extern crate tokio_executor;
extern crate either;
extern crate multiaddr;

extern crate mercury_connect;
extern crate mercury_storage;
extern crate mercury_home_protocol;



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
use tokio_timer::*;

use mercury_connect::*;
use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::service::{ConnectService, DummyUserInterface, ProfileGatewayFactory, ServiceImpl, SignerFactory};
use mercury_home_protocol::*;
use mercury_home_protocol::crypto::Ed25519Signer;
use mercury_storage::async::imp::InMemoryStore;
use application::{Application, EX_OK, EX_SOFTWARE, EX_USAGE};
use cli::cli;
use client::Client;
use client_config::*;
use function::*;
use logging::start_logging;
use server::Server;
use server_config::*;



fn temporary_connect_service_instance(my_private_profilekey_file: &str,
        home_id_str: &str, home_addr_str: &str, handle: reactor::Handle)
    -> Result<(Rc<ConnectService>, ProfileId, ProfileId), std::io::Error>
{
    debug!("Initializing service instance");

    let home_pubkey = PublicKey(std::fs::read(home_id_str)?);
    let home_id = ProfileId::from(&home_pubkey);
    let home_addr :SocketAddr = home_addr_str.parse().map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    let home_multiaddr : Multiaddr = home_addr.to_multiaddr().expect("Failed to parse server address");
    let home_profile = Profile::new_home( home_id.clone(), home_pubkey.clone(), home_multiaddr );

    // TODO consider that client should be able to start up without being a DHT client,
    //      e.g. with having only a Home URL including hints to access Home
    let mut profile_repo = SimpleProfileRepo::new();
    profile_repo.insert(home_profile);
    let profile_repo = Rc::new(profile_repo);

    let private_key = PrivateKey(std::fs::read(my_private_profilekey_file)?);
    let client_signer = Rc::new( Ed25519Signer::new(&private_key).unwrap() ) as Rc<Signer>;
    let client_id = client_signer.profile_id().to_owned();
    let signers = vec![ ( client_id.clone(), client_signer ) ].into_iter().collect();

    let signer_factory: Rc<SignerFactory> = Rc::new(SignerFactory::new(signers) );
    let home_connector = Rc::new( SimpleTcpHomeConnector::new( handle.clone() ) );
    let gateways = Rc::new( ProfileGatewayFactory::new(
        signer_factory, profile_repo.clone(), home_connector ) );

    let my_profiles = Rc::new( HashSet::new() );
    let ui = Rc::new( DummyUserInterface::new( my_profiles.clone() ) );
    let profile_store = Rc::new( RefCell::new( InMemoryStore::new() ) );
    let service = Rc::new( ServiceImpl::new(ui, my_profiles, profile_store, gateways, &handle) );

    Ok( (service, client_id, home_id) )
}



fn temporary_init_env(service: Rc<ConnectService>, client_id: ProfileId, home_id: ProfileId)
    -> Box< Future<Item=(), Error=std::io::Error> >
{
    let init_fut = service.admin_endpoint(None)
        .inspect( |_admin| debug!("Admin endpoint was connected") )
        .and_then( move |admin| admin.join_home(&client_id, &home_id) )
        .inspect( |_| debug!("Successfully joined to Home") )
        .map_err( |e| std::io::Error::new( std::io::ErrorKind::ConnectionRefused, format!("{}", e) ) );
    Box::new(init_fut)
}



pub struct AppContext{
    service: Rc<ConnectService>,
    client_id: ProfileId,
    home_id: ProfileId
}

impl AppContext
{
    pub fn new(priv_key: &str, node_id: &str, node_addr: &str, handle: reactor::Handle)
        -> Result<Self, std::io::Error>
    {
        // TODO when we'll have a standalone service with proper IPC/RPC interface,
        //      this must be changed into a simple connect() call instead of building a service instance
        let (service, client_id, home_id) = temporary_connect_service_instance(priv_key, node_id, node_addr, handle)?;
        Ok( Self{service, client_id, home_id} )
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

    // Initialize logging
    match matches.occurrences_of(cli::CLI_VERBOSE) {
        0 => start_logging("i"),
        1 => start_logging("d"),
        _ => start_logging("t"),
    }

    // Creating a reactor
    let mut reactor = reactor::Core::new().unwrap();

    // Constructing application context from command line args
    let appcx = AppContext::new(
        matches.value_of(cli::CLI_PRIVATE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_HOME_NODE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_SERVER_ADDRESS).unwrap(),
        reactor.handle() )?;

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

    // SIGINT is terminating the server
    let sigint_fut = signal_recv(SIGINT).into_future()
        .map(|_| info!("received SIGINT, terminating application") )
        .map_err(|(err, _)| err);

    reactor.run(app_fut.select(sigint_fut).map(|(item, _)| item).map_err(|(err, _)| err))
}

fn main() {
    Application::run(application_code());
}
