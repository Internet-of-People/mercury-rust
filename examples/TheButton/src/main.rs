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

extern crate mercury_connect;
extern crate mercury_storage;
extern crate mercury_home_protocol;

pub mod client_config;
pub mod client;
pub mod server_config;
pub mod server;
pub mod logging;
pub mod function;
pub mod application;
// pub mod signal_handling;

use client_config::*;
use server_config::*;
use function::*;
use server::Server;
use client::Client;
use logging::start_logging;
use std::net::SocketAddr;
use futures::IntoFuture;
use application::{Application, EX_OK, EX_SOFTWARE, EX_UNAVAILABLE, EX_TEMPFAIL};

use clap::{App, ArgMatches};

use futures::{Async, Future};

use tokio_uds::*;
use tokio_io::io::read;
use tokio_core::reactor::Core;
use tokio_timer::*;
use tokio_signal::unix::{SIGINT, SIGUSR1, SIGUSR2};

use mercury_connect::sdk::{DAppApi, Call};
use mercury_connect::{Relation};
use mercury_home_protocol::*;
use mercury_storage::{async::KeyValueStore};


pub struct AppContext{
    priv_key: PrivateKey,
    home_node: ProfileId,
    home_address: SocketAddr,
}

impl AppContext{
    pub fn new(priv_key: &str, node_id: &str, node_addr: &str)->Result<Self, std::io::Error>{
        let key = PrivateKey(priv_key.into());
        let prof = ProfileId(node_id.into());
        let addr = node_addr.parse().map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        Ok(Self{
            priv_key: key,
            home_node: prof,
            home_address: addr,
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
        Err(err) => 
            match err.kind() {
                _ => EX_SOFTWARE
            }
    }
}

fn application_code_internal() -> Result<(), std::io::Error> {
    //ARGUMENT HANDLING START
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    //VERSION
    if matches.is_present("version"){
        println!("The Button Dapp >>> version: 0.1 pre-alpha");
    }
    //VERBOSITY HANDLING
    match matches.occurrences_of("verbose") {
        0 => {
            start_logging("o");
            println!("verbose 0: logging off")
        },
        1 => {
            start_logging("w");
            warn!("verbose 1: logging warn")
        },
        2 => {
            start_logging("i");
            info!("verbose 2: logging info")
        },
        3 | _ => {
            start_logging("d");
            debug!("verbose 3 or more: debug")
        },
    }
    //GET APPLICATION CONTEXT    
    let appcx = AppContext::new(
        matches.value_of("private-key").unwrap(), 
        matches.value_of("home-node-public").unwrap(), 
        matches.value_of("home-node-address").unwrap())?;
    let prof_rep = mercury_connect::SimpleProfileRepo::new();

    //SERVER MODE HANDLING
    let (sub_name, sub_args) = matches.subcommand();
    
    let app_mode = match sub_args {
        Some(args)=>{
            match sub_name{
                "server"=>{
                    ServerConfig::new_from_args(args.to_owned())
                        .map( |cfg|
                            unimplemented!()
                            //Mode::Server(Server::new(cfg, DAppConnect::new(unimplemented!())))
                        )
                },
                "client"=>{
                    ClientConfig::new_from_args(args.to_owned())
                        .map( |cfg| 
                            Mode::Client(Client::new(cfg, appcx))
                        )
                },
                _=>{
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse subcommand"))
                }
            }
        },
        None=>{
            warn!("No subcommand given, starting in server mode");
            //Ok(Mode::Server(Server::default(DAppConnect::new(unimplemented!()))))
            unimplemented!()
        }
    };

    //TOKIO RUN
    //TODO expand errors if needed
    let mut reactor = Core::new().unwrap();

    let app_fut = match app_mode? {
        Mode::Client(client_fut) => 
            Box::new(client_fut.into_future()),

        Mode::Server(server_fut) => 
            Box::new(server_fut.into_future()),  
    };

    reactor.run(app_fut)
}

fn main() {
    Application::run(application_code());
}
