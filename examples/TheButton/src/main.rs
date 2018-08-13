#[macro_use]
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

use cli::cli;
use function::*;
use server::Server;
use client::Client;
use client_config::*;
use server_config::*;
use logging::start_logging;
use application::{Application, EX_OK, EX_SOFTWARE, EX_USAGE};

use std::rc::Rc;
use std::net::SocketAddr;

use clap::{App, ArgMatches};

use futures::Future;
use futures::{IntoFuture, Stream};

use tokio_signal::unix::SIGINT;
use tokio_core::reactor::{Core, Handle};
use tokio_timer::*;

use multiaddr::{Multiaddr, ToMultiaddr};

use mercury_connect::*;
use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::sdk::{DAppInit, DAppApi};
use mercury_connect::{SimpleProfileRepo, ProfileGatewayImpl, ProfileGateway};
use mercury_home_protocol::*;
use mercury_home_protocol::AppMessageFrame;
use mercury_home_protocol::crypto::Ed25519Signer;


pub struct AppContext{
    priv_key: PrivateKey,
    home_pub: PublicKey,
    home_address: SocketAddr,
    gateway: Rc<ProfileGateway>,
    handle: Handle,
}

impl AppContext{
    pub fn new(priv_key: &str, node_id: &str, node_addr: &str, handle: Handle)->Result<Self, std::io::Error>{
        let server_pub = PublicKey(std::fs::read(node_id)?);
        let private_key = PrivateKey(std::fs::read(priv_key)?);
        let server_id = ProfileId::from(&server_pub);

        let addr :SocketAddr = node_addr.parse().map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        let multaddr : Multiaddr = addr.clone().to_multiaddr().expect("Failed to parse server address");
        
        let client_signer = Rc::new( Ed25519Signer::new(&private_key).unwrap() );
        let mut profile_store = SimpleProfileRepo::new();
        let home_connector = SimpleTcpHomeConnector::new( handle.clone() );
        let home_profile = Profile::new_home(
            server_id, 
            server_pub.clone(), 
            multaddr
        );
        profile_store.insert(home_profile);
        
        let profile_gw = Rc::new(ProfileGatewayImpl::new(client_signer, Rc::new(profile_store),  Rc::new(home_connector)));

        Ok(Self{
            priv_key: private_key,
            home_pub: server_pub,
            home_address: addr,
            gateway: profile_gw,
            handle
        })
    }
}

#[derive(Debug)]
pub enum OnFail {
    Terminate,
    Retry,
}

enum Mode{
    Server(Server),
    Client(Client)
}

fn application_code() -> i32 {
    match application_code_internal() {
        Ok(_) => 
            EX_OK,
        Err(err) => {       
            error!("application failed: {}", err);
            match err.kind() {
                std::io::ErrorKind::InvalidInput => EX_USAGE,
                _ => EX_SOFTWARE
            }
        }
    }
}

fn application_code_internal() -> Result<(), std::io::Error> {
    //ARGUMENT HANDLING START
    let matches = cli().get_matches();

    // Print version
    if matches.is_present(cli::CLI_VERSION){
        println!("The Button dApp 0.1 pre-alpha");
        return Ok(())
    }

    // Initialize logging
    match matches.occurrences_of(cli::CLI_VERSION) {
        1 => start_logging("d"),
        2 => start_logging("t"),
        0|_ => start_logging("i"),                
    }

    // Creating a reactor
    let mut reactor = Core::new().unwrap();

    // Constructing application context from command line args
    let appcx = AppContext::new(
        matches.value_of(cli::CLI_PRIVATE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_HOME_NODE_KEY_FILE).unwrap(), 
        matches.value_of(cli::CLI_SERVER_ADDRESS).unwrap(),
        reactor.handle())?;

    // Creating application object
    let (sub_name, sub_args) = matches.subcommand();
    let app_mode = match sub_args {
        Some(args)=>{
            match sub_name{
                cli::CLI_SERVER => 
                    ServerConfig::new_from_args(args.to_owned())
                        .map( |cfg|
                            Mode::Server(Server::new(cfg, appcx))
                        ),
                cli::CLI_CLIENT => 
                    ClientConfig::new_from_args(args.to_owned())
                        .map( |cfg| 
                            Mode::Client(Client::new(cfg, appcx))
                        ),
                _=> 
                    return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("unknown subcommand '{}'", sub_name)))
                
                
            }
        },
        _=> 
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "subcommand missing"))
    };

    // Running the application

    let app_fut = match app_mode? {
        Mode::Client(client_fut) => 
            Box::new(client_fut.into_future()),
        Mode::Server(server_fut) => 
            Box::new(server_fut.into_future()),  
    };

    // SIGINT is terminating the server
    let sigint_fut = signal_recv(SIGINT).into_future()
        .map(|_| {
            info!("received SIGINT, terminating application");
            ()
        })
        .map_err(|(err, _)| err);

    reactor.run(app_fut.select(sigint_fut).map(|(item, _)| item).map_err(|(err, _)| err))
}

fn main() {
    Application::run(application_code());
}
