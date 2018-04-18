#![allow(unused)]
extern crate mercury_connect;
extern crate mercury_home_protocol;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;

use ::net::*;
use ::dummy::*;

use mercury_connect::*;
use mercury_home_protocol::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::io::{BufRead, Read, Write, stdin};

use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

use futures::{Future, Stream};

fn main(){
    //print!("{}[2J", 27 as char);
    println!("Setting up config");
    let mut reactor = reactor::Core::new().unwrap();
    let reactorhandle = reactor.handle();
    let homeaddr = "/ip4/127.0.0.1/udp/9876";
    let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    
    println!("Setting up signers");
    let signo = Rc::new(dummy::Signo::new("Deusz"));
    let homesigno = Rc::new(dummy::Signo::new("makusguba"));
    
    println!("Setting up home");

    let homeprof = Profile::new_home(homesigno.prof_id().to_owned(), homesigno.pub_key().to_owned(), homemultiaddr.clone());
    let profile = make_own_persona_profile(signo.pub_key());
    
    println!("Setting up connection");

    let mut dht = ProfileStore::new();
    dht.insert(homeprof.id.clone(), homeprof.clone());
    let mut home_storage = Rc::new( RefCell::new(dht) );
    let mut store_rc = Rc::clone(&home_storage);
    let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , home_storage ) ) );

    let profilegateway = ProfileGatewayImpl{
        signer:         signo,
        profile_repo:   store_rc,
        home_connector: Rc::new( dummy::DummyConnector::new_with_home( home ) ),
    };

    println!("Registering");
    let reg = profilegateway.register(homesigno.prof_id().to_owned(), dummy::create_ownprofile( profile ), None);
    let ownprofile = reactor.run(reg).unwrap();
    println!("{:?}", ownprofile );
    
    println!("Logging in");
    println!("Getting session");

    let session = reactor.run( profilegateway.login() ).unwrap();
    
    println!("All set up");
    
    println!("Menu\n1. Connect\n2. Call(crashes)\n3. Pair\n4. Ping\n5. Show profile\nExit with ctrl+d");
    let stdin = tokio_stdin_stdout::stdin(1);
    let bufreader = std::io::BufReader::new(stdin);
    let instream = tokio_io::io::lines(bufreader);
    let stdin_closed = instream.for_each(|line|{     
        match line.as_ref(){
            "1" =>{
                let signer = profilegateway.signer.to_owned();
                profilegateway.home_connector.connect(&homeprof, signer);
                println!("connect");
    
            },
            "2" =>{
                profilegateway.call(
                    dummy::dummy_relation("work"), 
                    ApplicationId( String::from("SampleApp") ), 
                    AppMessageFrame("whatever".as_bytes().to_owned() ) 
                );
    
            }
            "3" =>{
                profilegateway.pair_request("relation_dummy_type", "url");
    
            }
            "4" =>{
                session.ping("dummy_ping");
    
            }
            "5" =>{
                println!("{:?}", ownprofile);
    
            }
            _ =>{
                println!("nope");
    
            },
        };
        futures::future::ok::<(),std::io::Error>(())
    });
    let crash = reactor.run(stdin_closed).unwrap();
}
