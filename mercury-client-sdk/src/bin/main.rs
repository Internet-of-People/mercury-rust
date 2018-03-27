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
use futures::Future;

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
    println!("Setting up config");
    let mut reactor = reactor::Core::new().unwrap();
    let mut reactorhandle = reactor.handle();
    let homeaddr = "/ip4/127.0.0.1/udp/9876";
    let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    
    println!("Setting up signers");
    let signo = Rc::new(mock::Signo::new("Deuszkulcs"));
    let homesigno = Rc::new(mock::Signo::new("makusguba"));
    
    println!("Setting up profiles");
    let homeprof = mock::make_home_profile(&homeaddr,"home","szeretem a kakaot");
    let profile = make_own_persona_profile("Deusz", signo.pub_key());
    
    println!("Setting up connection");
    let capclient = TcpStream::connect( &multiaddr_to_socketaddr(&homemultiaddr).unwrap() , &reactorhandle.clone() )
    .map(|stream|{
        let cap = Rc::new(HomeClientCapnProto::new(
            stream,
            Box::new(HomeContext::new(signo.clone(), &homeprof)),
            reactor.handle()
        ));
        AppContext{
            profilegateway : Box::new(
                ProfileGatewayImpl{
                    signer:         signo,
                    profile_repo:   cap,
                    home_connector: Rc::new(SimpleTcpHomeConnector::new(reactorhandle)),
        })}
    });
    let mut reactor2 = reactor::Core::new().unwrap();
    let appcontext = reactor2.run(capclient).unwrap();
    
    println!("Menu\n1. Connect\n2. Login\n3. Call(crashes)\n4. Register\n5. Pair\n0. Exit");
    loop{
        let mut buffer = String::new();
        let stdin = stdin();
        let mut handle = stdin.lock();
        handle.read_line(&mut buffer);
        match buffer.as_ref(){
            "1\n" =>{
                let signer = appcontext.profilegateway.signer.to_owned();
                appcontext.profilegateway.home_connector.connect(&homeprof, signer);
                println!("connect");
            },
            "2\n" =>{
                appcontext.profilegateway.login();
            }
            //call dies miserably 
            "3\n" =>{
                appcontext.profilegateway.call(
                    mock::dummy_relation("work"), 
                    ApplicationId( String::from("SampleApp") ), 
                    AppMessageFrame("whatever".as_bytes().to_owned() ) 
                );
            }
            "4\n" =>{
                appcontext.profilegateway.register(ProfileId("Home".as_bytes().to_owned()),mock::create_ownprofile("Deusz"),None);
            }
            "5\n" =>{
                appcontext.profilegateway.pair_request("relation_dummy_type", "url");
            }
            "0\n" =>{
                break;
            }
            _ =>{println!("nope");},
        }
    }
}
