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



// fn re(){
//     let mut reactor = reactor::Core::new().unwrap();
//     let reactorhandle = reactor.handle();
//     let pk = &profile.signer.pub_key().0;
//     let test_fut = TcpStream::connect( &multiaddr_to_socketaddr(&homemultiaddr).unwrap(), &reactorhandle )
//         .map_err( |_e| ErrorToBeSpecified::TODO )
//         .and_then( move |tcp_stream|
//         {
//             let home = HomeClientCapnProto::new(tcp_stream, reactorhandle);
//             home.login(profile.to_owned())
//         } )
//         .and_then( |session| session.ping( "std::str::from_utf8(&pk).unwrap()" ) );
// 
//     let pong = reactor.run(test_fut);
//     println!("Response: {:?}", pong);
// 
// }
// let mut reactor = reactor::Core::new().unwrap();
// let reactorhandle = reactor.handle();
// let test_fut = TcpStream::connect( &multiaddr_to_socketaddr(&homemultiaddr).unwrap(), &reactorhandle )
//     .map_err( |_e| ErrorToBeSpecified::TODO )
//     .and_then( move |tcp_stream|{
//         let home = HomeClientCapnProto::new( tcp_stream, dummyhome, reactorhandle );
//         home.login(ProfileId("Deusz".as_bytes().to_owned()))
//     } )
//     .and_then( |session| session.ping( "std::str::from_utf8(&pk).unwrap()" ) );
// 
// let pong = reactor.run(test_fut);
// println!("Response: {:?}", pong);

fn main(){
    //print!("{}[2J", 27 as char);
    let homeaddr = "/ip4/127.0.0.1/udp/9876";
    let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    let homeprof = mock::make_home_profile(&homeaddr,"home","");
    let signo = Rc::new(mock::Signo::new("Daswitch"));
    let homesigno = Rc::new(mock::Signo::new("makusguba"));
    let prof_rep = mock::DummyHome::new("pong");
    let home_rep = mock::DummyHome::new("home");
    let dummyhome = Box::new(mock::DummyHome::new("homedummy"));
    let dummyhome2 = mock::DummyHome::new("homedummy2");
    let homecontext = HomeContext::new(homesigno, &homeprof);
    let connect = ConnectApp{ home : mock::DummyHome::new("apples") };
    
    let profile = make_own_persona_profile("Deusz", signo.pub_key());
    
    let appcontext = AppContext{
        profilegateway : Box::new(
            ProfileGatewayImpl{
                signer:         signo,
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
                let signer = appcontext.profilegateway.signer.to_owned();
                appcontext.profilegateway.home_connector.connect(&homeprof, signer);
                println!("connect");
            },
            "login\n" =>{
                appcontext.profilegateway.login();
            }
            "call\n" =>{
                dummyhome.call(mock::dummy_relation("work"), ApplicationId( String::from("SampleApp") ), AppMessageFrame("whatever".as_bytes().to_owned() ) );
                unimplemented!();
            }
            "register\n" =>{
                dummyhome.register(mock::create_ownprofile("Deusz"),None);
            }
            "pair\n" =>{
                
            }
            _ =>{println!("nope");},
        }
    }



}
