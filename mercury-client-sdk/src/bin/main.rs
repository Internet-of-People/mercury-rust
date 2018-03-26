#![allow(unused)]
extern crate mercury_sdk;
extern crate mercury_common;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_core;
extern crate tokio_io;
extern crate futures;


use std::rc::Rc;
use std::io::{BufRead, Read, Write, stdin};

use mercury_common::*;
use mercury_sdk::*;
use ::net::*;
use ::mock::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

struct ConnectApp{
    home    : DummyHome,
}

impl ConnectApp{
    
}

struct AppContext{
    profilegateway : Box<ProfileGatewayImpl>,
}

impl AppContext{
    
}

fn main(){
    //print!("{}[2J", 27 as char);
    let signo = mock::Signo::new("Daswitch");
    let prof_rep = mock::DummyHome::new("pong");
    let home_rep = mock::DummyHome::new("home");
    let home = mock::DummyHome::new("home");
    let connect = ConnectApp{ home : mock::DummyHome::new("apples") };
    let appcontext = AppContext{
        profilegateway : Box::new(
            ProfileGatewayImpl{
                signer:         Rc::new(signo),
                profile_repo:   Rc::new(prof_rep),
                home_connector: Rc::new(DummyHomeConnector{home: home_rep}),
    })};
    loop{
        let mut buffer = String::new();
        //let mut buffer = vec!();
        let stdin = stdin();
        let mut handle = stdin.lock();
        handle.read_line(&mut buffer);
        match buffer.as_ref(){
            "connect\n" =>{
                //appcontext.profilegateway.home_connector.dconnect();
                println!("connect" );
            },
            "login\n" =>{
                appcontext.profilegateway.login();
            }
            "call\n" =>{
                home.call();
            }
            "register\n" =>{
                home.register(mock::create_ownprofile("Deusz"),None);
            }
            _ =>{println!("nope");},
        };
    }
}
