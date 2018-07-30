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
extern crate mercury_home_protocol;

pub mod config;
pub mod client;
pub mod server;
pub mod logging;
pub mod function;
pub mod application;
// pub mod mercury_wire;
// pub mod signal_handling;

use mercury_home_protocol::{PrivateKey, ProfileId};

use config::*;
use function::*;
use server::Server;
use client::Client;
use logging::start_logging;
use application::{Application, EX_OK, EX_SOFTWARE, EX_UNAVAILABLE, EX_TEMPFAIL};

use clap::{App, ArgMatches};

use futures::{Async, Future};

use tokio_uds::*;
use tokio_io::io::read;
use tokio_core::reactor::Core;
use tokio_timer::*;
use tokio_signal::unix::{SIGINT, SIGUSR1, SIGUSR2};

struct AppContext{
    priv_key: PrivateKey,
    home_node: ProfileId,
    home_address: String,
}

impl AppContext{
    pub fn new(priv_key: Option<&str>, node_id: Option<&str>, node_addr: Option<&str>)->Result<Self, std::io::Error>{
        let key : PrivateKey;
        match priv_key {
            Some(k) => {key = PrivateKey(k.into());},
            None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "no key")),
        };
        let prof : ProfileId; 
        match node_id {
            Some(id) => {prof = ProfileId(id.into());},
            None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "no id")),
        };
        let addr;
        match node_addr {
            Some(naddr) => {addr = naddr.to_string();},
            None => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "no id")),
        };
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
            0,
        Err(err) =>
        //TODO
            match err{
                _=>42
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
    let appcx;
    match AppContext::new(  matches.value_of("private-key"), 
                            matches.value_of("home-node-public"), 
                            matches.value_of("home-node-address")){
        Ok(cx) => {appcx = cx;},
        Err(e) => return Err(e)
    };

    //SERVER MODE HANDLING
    let (sub_name, sub_args) = matches.subcommand();
    
    let app_mode = match sub_args {
        Some(args)=>{
            match sub_name{
                "server"=>{
                    ServerConfig::new_from_args(args.to_owned())
                        .map( |cfg| 
                            Mode::Server(Server::new(cfg))
                        )
                },
                "client"=>{
                    ClientConfig::new_from_args(args.to_owned())
                        .map( |cfg| 
                            Mode::Client(Client::new(cfg))
                        )
                },
                _=>{
                    Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse subcommand"))
                }
            }
        },
        None=>{
            warn!("No subcommand given, starting in server mode");
            Ok(Mode::Server(Server::default()))
        }
    };

    //TOKIO RUN
    //TODO expand errors if needed
    let mut reactor = Core::new().unwrap();

    let app_fut = match app_mode? {
        Mode::Client(client_fut) => 
            Box::new(client_fut) as Box<Future<Item=i32, Error=std::io::Error>>,

        Mode::Server(server_fut) => 
            Box::new(server_fut) as Box<Future<Item=i32, Error=std::io::Error>>,  
    };

    match reactor.run({
            app_fut
            .map(|_|return EX_OK)
            .map_err(|err| {
                match err.kind(){
                    std::io::ErrorKind::Interrupted => {
                        warn!("exiting on SIGINT");
                        return EX_OK;
                    }std::io::ErrorKind::NotConnected => {
                        return EX_UNAVAILABLE;
                    }std::io::ErrorKind::TimedOut => {
                        return EX_TEMPFAIL;
                    }_=>{
                        return EX_SOFTWARE;
                    }
                }
            })
    }){
        Ok(code)=>code,
        Err(code)=>code   
    };
    Ok(())
}

fn main() {
    Application::run(application_code());
}
