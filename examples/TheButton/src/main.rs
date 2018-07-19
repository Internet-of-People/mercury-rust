#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate log4rs;

extern crate futures;
extern crate tokio;
extern crate tokio_core;
extern crate tokio_signal;
extern crate mercury_connect;
extern crate tokio_uds;

pub mod config;
pub mod client;
pub mod server;
pub mod logging;
pub mod function;
pub mod application;

use config::*;
use function::*;
use server::Server;
use client::Client;
use logging::start_logging;
use application::{Application, EX_OK, EX_USAGE, EX_SOFTWARE, EX_UNAVAILABLE, EX_TEMPFAIL};

use clap::{App, ArgMatches};

use futures::{future, Future, Stream};

use tokio_uds::*;
use tokio::io::read_to_end;
use tokio_core::reactor::Core;
use tokio_signal::unix::{SIGINT, SIGUSR1, SIGUSR2};

#[derive(Debug)]
pub enum OnFail {
    TERMINATE,
    RETRY,
}

enum Mode{
    SERVER(Server),
    CLIENT(Client),
    ERROR(i32),
}

fn application_code()->i32{
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
    // LOGGING START
    debug!("Starting TheButtonDapp...");

    //SERVER MODE HANDLING
    let (sub_name, sub_args) = matches.subcommand();
    let app_mode : Mode;
    match sub_args {
        Some(args)=>{
            match sub_name{
                "server"=>{
                    let cfg = ServerConfig::new_from_args(args.to_owned());
                    app_mode = Mode::SERVER(Server::new(cfg));
                },
                "client"=>{
                    let cfg = ClientConfig::new_from_args(args.to_owned());
                    app_mode = Mode::CLIENT(Client::new(cfg));
                },
                _=>{
                    warn!("Subcommand could not be parsed");
                    app_mode = Mode::ERROR(EX_USAGE);
                }
            }
        }
        None=>{
            warn!("No subcommand given, starting in server mode");
            app_mode = Mode::SERVER(Server::default());
        }
    };

    // SIGNAL HANDLER STREAMS
    let c = signal_recv(SIGINT).for_each(|_| {
        info!("SIGINT received!");
        Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "SIGINT"))
    });

    let u1 = signal_recv(SIGUSR1).for_each(|_| {
        info!("SIGUSR1 received!");
        Server::generate_event();
        Ok(())
    });

    let u2 = signal_recv(SIGUSR2).for_each(|_| {
        info!("SIGUSR2 received!");
        Server::stop_event_generation();
        Ok(())
    });

    //TOKIO RUN
    //TODO expand errors if needed
    let mut reactor = Core::new().unwrap();
    
    let core_fut = Future::join3(c,u1,u2);

    if let Mode::SERVER(ser) = app_mode {
        match reactor.run(
            core_fut.join(ser)
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
        ){
            Ok(code)=>code,
            Err(code)=>code   
        }
    }else if let Mode::CLIENT(cli) = app_mode{
        match reactor.run(
            core_fut.join(cli)
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
        ){
            Ok(code)=>code,
            Err(code)=>code   
        }
    }else if let Mode::ERROR(e) = app_mode{
        return e
    }else{
        return EX_SOFTWARE;
    }
}

fn main() {
    Application::run(application_code());
}
