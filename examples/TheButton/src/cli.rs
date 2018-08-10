use super::*;

use clap::{App, Arg, SubCommand, AppSettings};

pub const CLI_VERSION : &str = "version";
pub const CLI_VERBOSE : &str = "verbose";
pub const CLI_PRIVATE_KEY_FILE : &str = "private-key-file";
pub const CLI_HOME_NODE_KEY_FILE : &str = "home-node-key-file";
pub const CLI_SERVER_ADDRESS : &str = "server-addr";
pub const CLI_SERVER : &str = "server";
pub const CLI_EVENT_TIMER : &str = "event-timer";
pub const CLI_STOP_AFTER : &str = "stop-after";
pub const CLI_CLIENT : &str = "client";
pub const CLI_CONNECT : &str = "connect";


pub fn cli<'a, 'b>()->App<'a, 'b>{
    App::new("TheButton")
        .setting(AppSettings::SubcommandRequired)
        .version("0.1")
        .author("Deusz <lovaszoltanr@gmail.com>")
        .about("The application has two modus operandi (selected via command line args) 
                Client - Connects to the server via the dApp SDK. 
                If connection failed, exits with an error code 
                or retries with exponential timeout or exits immediately 
                (controlled by a command line switch). 

                Server - Able to provide notifications to connected clients in case of events. 
                Events can be raised via a timer, or via signal (SIGUSR1).")
        .arg(Arg::with_name(CLI_VERSION)
            .short("v")
            .long(CLI_VERSION)
            .multiple(true)
            .help("Sets the level of verbosity")
        )
        .arg(Arg::with_name(CLI_VERSION)
            .short("V")
            .long(CLI_VERSION)
            .help("software version")
        )
        .arg(Arg::with_name(CLI_PRIVATE_KEY_FILE)
            .long(CLI_PRIVATE_KEY_FILE)
            .takes_value(true)
            .help("private key of the client (binary, ed25519)")
            .default_value("../../etc/client.id")
            .value_name("KEY")
        )    
        .arg(Arg::with_name(CLI_HOME_NODE_KEY_FILE)
            .long(CLI_HOME_NODE_KEY_FILE)
            .takes_value(true)
            .help(" public key of the server (binary, ed25519)")
            .default_value("../../etc/homenode.id.pub")
            .value_name("KEY")
        )
        .arg(Arg::with_name(CLI_SERVER_ADDRESS)
            .long(CLI_SERVER_ADDRESS)
            .takes_value(true)
            .help("ipv4 address of the server")
            .value_name("ADDRESS")
            .default_value("127.0.0.1:2077")
        )
        .subcommand(SubCommand::with_name(CLI_SERVER)
            .about("Sets running mode to server")
            .arg(Arg::with_name("event-file")
                .long("event-file")
                .takes_value(true)
                .value_name("PATH")
                .help("path name of device file to poll (every byte on the stream generates an event)")
            )
            .arg(Arg::with_name(CLI_EVENT_TIMER)
                .long(CLI_EVENT_TIMER)
                .takes_value(true)
                .value_name("GENTIMER")
                .help("takes n, generating an event automatically every n milliseconds")
            )
            .arg(Arg::with_name(CLI_STOP_AFTER)
                .long(CLI_STOP_AFTER)
                .takes_value(true)
                .value_name("STOPCOUNT")
                .help("takes n, when set the server exits after providing n events")
            )
        )
        .subcommand(SubCommand::with_name(CLI_CLIENT)
            .about("Sets running mode to client")
            .arg(Arg::with_name("on-fail")
                .long("on-fail")
                .takes_value(true)
                .value_name("FAIL")
                .help("terminate (default) - 
                            the client terminates execution on connection errors
                        retry - 
                            do an exponential reconnection from 1 sec up to 32 secs ")
            )
            .arg(Arg::with_name(CLI_CONNECT)
                .long(CLI_CONNECT)
                .takes_value(true)
                .required(true)
                .value_name("PROFILE_ID")
                .help("profile id of the button app server")
            )
        )
}