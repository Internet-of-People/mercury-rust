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

fn main(){
    //print!("{}[2J", 27 as char);
    println!( "Setting up config" );
    let mut reactor = reactor::Core::new().unwrap();
    let mut reactorhandle = reactor.handle();
    let homeaddr = "/ip4/127.0.0.1/udp/9876";
    let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    
    println!( "Setting up signers" );
    let signo = Rc::new( mock::Signo::new( "Deuszkulcs" ) );
    let other_signo = Rc::new( mock::Signo::new( "Othereusz" ) );
    let homesigno = Rc::new( mock::Signo::new( "makusguba" ) );
    let other_homesigno = Rc::new( mock::Signo::new( "tulfozotttea" ) );
    println!("Setting up profiles");
    let homeprof = mock::make_home_profile( &homeaddr,"home","szeretem a kakaot" );
    let other_homeprof = mock::make_home_profile( &homeaddr,"otherhome","konyhalevel100" );
    let mut profile = make_own_persona_profile( "Deusz", signo.pub_key() );
    let mut other_profile = make_own_persona_profile( "Othereusz", signo.pub_key() );

    // let cap = Rc::new(HomeClientCapnProto::new(
    //     TcpStream::connect( 
    //         &multiaddr_to_socketaddr( &homemultiaddr ).unwrap(),
    //         &reactorhandle.clone() 
    //     ),
    //     Box::new(HomeContext::new(signo.clone(), &homeprof)),
    //     reactorhandle.clone()
    // ));
    println!( "Own Gateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );
    let own_gateway = ProfileGatewayImpl::new(
        signo,
        Rc::new(mock::DummyHome::new("eins")),
        Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
    );


    println!( "any_home_of(profile) -> Home" );
    let bizbasz = own_gateway.any_home_of(&profile)

    // let bizbasz = TcpStream::connect( 
    //     &multiaddr_to_socketaddr( &homemultiaddr ).unwrap(),
    //     &reactorhandle.clone() 
    // ).map(|stream|{
    //     println!( "Setting up connection" );
    //     println!( "Own Proto: TcpStream, HomeContext, ReactorHandle" );
    //     let cap = Rc::new(HomeClientCapnProto::new(
    //         stream,
    //         Box::new(HomeContext::new(signo.clone(), &homeprof)),
    //         reactorhandle.clone()
    //     ));
        // println!( "Other Proto: TcpStream, HomeContext, ReactorHandle" );
        // let other_cap = Rc::new(HomeClientCapnProto::new(
        //     stream,
        //     Box::new(HomeContext::new(signo.clone(), &homeprof)),
        //     reactorhandle.clone()
        // ));
        // println!( "Own Gateway: ProfileSigner, CapnprotoClientSide(as profile repo), HomeConnector" );
        // let own_gateway = 
        // ProfileGatewayImpl{
        //     signer:         signo,
        //     profile_repo:   cap,
        //     home_connector: Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ),
        // }
        // println!( "Other Gateway: ProfileSigner, CapnprotoClientSide(as profile repo), HomeConnector" );
        // let other_gateway = ProfileGatewayImpl{
        //     signer:         other_signo,
        //     profile_repo:   cap,
        //     home_connector: Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ),
        // };
        
    .and_then(|home|{
        println!( "register(HomeProfile_Id_WhereWeRegister, OwnProfile) -> OwnProfile_ExtendedWithNewHome" );
        
        Ok(own_gateway.register(
            ProfileId( Vec::from("Home") ),
            mock::create_ownprofile( "Deusz" ),
            None
        ))
    })
    .and_then(|ownprofile|{
        println!( "login() -> HomeSession" );
        own_gateway.login()
        
    }).and_then(| session |{
        println!( "ping(str) -> String" );
        
        session.ping( "dummy_ping" )
    }).and_then(|response|{
        println!( "request pair() -> (gives back nothing or error){:?}", response );
        
        own_gateway.pair_request( "relation_dummy_type", "url" )
        
    }).and_then(|()|{
        println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

        own_gateway.call(
            mock::dummy_relation( "work" ), 
            ApplicationId( String::from( "SampleApp" ) ), 
            AppMessageFrame( Vec::from( "whatever" ) ) 
        )        
    
    });
    
    println!( "All set up" );
    reactor.run( bizbasz );
    
    println!( "We're done here, let's go packing" );
}
