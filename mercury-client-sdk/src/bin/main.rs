#![allow(unused)]
extern crate mercury_sdk;
extern crate mercury_common;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
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
use futures::{Future,Stream};

struct ConnectApp{
    home    : DummyHome,
}

impl ConnectApp{
    
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
    let othersigno = Rc::new(mock::Signo::new("Othereusz"));
    let homesigno = Rc::new(mock::Signo::new("makusguba"));
    
    println!("Setting up profiles");
    let homeprof = mock::make_home_profile(&homeaddr,"home","szeretem a kakaot");
    let mut profile = make_own_persona_profile("Deusz", signo.pub_key());
    
    let bizbasz = TcpStream::connect( 
        &multiaddr_to_socketaddr(&homemultiaddr).unwrap(),
        &reactorhandle.clone() 
     ).and_then(|stream|{
        println!("Setting up connection");
        let cap = Rc::new(HomeClientCapnProto::new(
            stream,
            Box::new(HomeContext::new(signo.clone(), &homeprof)),
            reactorhandle.clone()
        ));
        let profilegateway = ProfileGatewayImpl{
            signer:         signo,
            profile_repo:   cap,
            home_connector: Rc::new(SimpleTcpHomeConnector::new(reactorhandle.clone())),
        };
    }).and_then(|profilegateway|{
        println!("register(HomeProfile_Id_WhereWeRegister, OwnProfile) -> OwnProfile_ExtendedWithNewHome");
        let ownprofile = profilegateway.register(ProfileId("Home".as_bytes().to_owned()),mock::create_ownprofile("Deusz"),None);
    }).and_then(|(profilegateway, ownprofile)|{
        println!("login() -> HomeSession");
        let session = profilegateway.login();
    }).and_then(|(profilegateway, session)|{
        println!("ping(str) -> String");
        
        session.ping("dummy_ping")
    }).and_then(|profilegateway|{
        println!("request pair() -> (gives back nothing or error)");
        
        let req = profilegateway.pair_request("relation_dummy_type", "url");
    }).and_then(|profilegateway|{
        println!("HomeConnector.connect(HomesProfile, OwnSigner) -> Home");
        
        let home = profilegateway.home_connector.connect(&homeprof, mock::Signo::new("Deuszkulcs"));
    }).and_then(|profilegateway|{
        println!("call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages");
        
        //let CallMessages = 
        profilegateway.call(
            mock::dummy_relation("work"), 
            ApplicationId( String::from("SampleApp") ), 
            AppMessageFrame(Vec::from("whatever")) 
        )        
    });
    println!("All set up");
    
    // println!("Menu\n1. Connect\n2. Call(crashes)\n3. Pair\n4. Ping\n5. Show profile\nExit with ctrl+d");
    //         "2" =>{
;
    // 
    //         }
    //         "3" =>{
    //             profilegateway.pair_request("relation_dummy_type", "url");


    reactor.run(bizbasz);
}
