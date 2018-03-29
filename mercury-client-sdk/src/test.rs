#[cfg(test)]
use super::*;

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

use ::net::*;
use ::mock::*;
use ::protocol_capnp::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};
use futures::{Future,Stream};

    #[test]
    fn test_register(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new(mock::DummyHome::new("Insomnia")),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let ownprofile = profile_gateway.register(
                ProfileId( Vec::from( "Insomnia" ) ),
                mock::create_ownprofile( "Noctis" ),
                None
        );

        let res = reactor.run(ownprofile);      
    }

    #[test]
    fn test_unregister(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new(mock::DummyHome::new("test_unregister")),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        // let home_prof = mock::make_home_profile( 
        //     "/ip4/127.0.0.1/udp/9876", 
        //     "Insomnia", 
        //     "FinalFantasyXV" 
        // );

        let registered = profile_gateway.register(
                ProfileId( Vec::from( "Insomnia" ) ),
                mock::create_ownprofile( "Noctis" ),
                None
        );
        let res = reactor.run(registered); 
        //assert

        let unregistered = profile_gateway.unregister(
                ProfileId( Vec::from( "Insomnia" ) ),
                ProfileId( Vec::from( "Noctis" ) ),
                None
        );

        let res = reactor.run(unregistered);      
    }

    #[test]
    fn test_login(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_login") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let home_session = profile_gateway.login();

        let res = reactor.run(home_session);      
    }

    #[test]
    fn test_claim(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_claim") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let home_session = profile_gateway.claim(
            ProfileId( Vec::from( "Insomnia" ) ),
            ProfileId( Vec::from( "Noctis" ) ),
        );

        let res = reactor.run(home_session);      
    }
    
    #[test]
    fn test_update(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_update") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let home_session = profile_gateway.update(
            ProfileId( Vec::from( "Tenebrae" ) ),
            &mock::create_ownprofile( "Noctis" ),
        );

        let res = reactor.run(home_session);      
    }

    #[test]
    fn test_any_home_of(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );

        let mut profile = make_own_persona_profile( "Chara", &signo.pub_key().clone() );

        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_any_home_of") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let home = profile_gateway.any_home_of(&profile);

        let res = reactor.run(home);        
    }

    #[test]
    fn test_any_home_of2(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_repo = Rc::new( mock::DummyHome::new("test_any_home_of") );
        let home_connector = Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) );

        let mut profile = make_own_persona_profile( "Chara", signo.pub_key() );

        let home = ProfileGatewayImpl::any_home_of2( 
            &profile, 
            profile_repo,
            home_connector, 
            signo
        );

        let res = reactor.run(home);
    }    

    #[test]
    fn test_call(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_call") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let call_messages = profile_gateway.call(
            mock::dummy_relation( "NewGame+" ), 
            ApplicationId( String::from( "Undertale" ) ), 
            AppMessageFrame( Vec::from( "Megalovania" ) ) 
        );

        let res = reactor.run(call_messages);      
    }

    #[test]
    fn test_ping(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_ping") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let response = profile_gateway.login()
        .and_then(|home_session|{
            home_session.ping( "test_ping" )
        });

        let res = reactor.run(response);      
    }

    //based on private method
    //  #[test]
    // fn test_new_half_proof(){
    //     let mut reactor = reactor::Core::new().unwrap();
    //     let mut reactorhandle = reactor.handle();
    //     let signo = Rc::new( mock::Signo::new( "TestKey" ) );

    //     let half_proof = ProfileGatewayImpl::new_half_proof( 
    //         "test",
    //         ProfileId( Vec::from("Chara") ),
    //         signo
    //     );

    //     let res = reactor.run(half_proof);      
    // }

    #[test]
    fn test_pair_req(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_pair_req") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let zero = profile_gateway.pair_request( "test_relation", "test_url" );

        let res = reactor.run(zero);   
    }

    #[test]
    fn test_pair_res(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_pair_res") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let zero = profile_gateway.pair_response( dummy_relation( "test_relation" ) );

        let res = reactor.run(zero);      
    }

    #[test]
    fn test_relations(){
        let mut reactor = reactor::Core::new().unwrap();
        let mut reactorhandle = reactor.handle();
        let signo = Rc::new( mock::Signo::new( "TestKey" ) );
        let profile_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("test_relations") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let zero = profile_gateway.relations( &ProfileId( Vec::from( "Noctis" ) ) );

        let res = reactor.run(zero);
    }

    #[test]
    fn and_then_story(){
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

        println!( "ProfileGateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );
        let own_gateway = ProfileGatewayImpl::new(
            signo,
            Rc::new( mock::DummyHome::new("ein") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        let other_gateway = ProfileGatewayImpl::new(
            other_signo,
            Rc::new( mock::DummyHome::new("zwei") ),
            Rc::new( SimpleTcpHomeConnector::new( reactorhandle.clone() ) ) 
        );

        println!( "any_home_of(profile) -> Home" );
        let ownapp = own_gateway.any_home_of(&profile)
        .and_then(|home|{
            println!( "register(HomeProfile_Id_WhereWeRegister, OwnProfile) -> OwnProfile_ExtendedWithNewHome" );
            
            Ok(own_gateway.register(
                ProfileId( Vec::from("Home") ),
                mock::create_ownprofile( "Deusz" ),
                None
            ))
        })
        .and_then(|ownprofile|{
            other_gateway.any_home_of(&profile)
        })
        .and_then(| otherhome |{
            println!( "register(HomeProfile_Id_WhereWeRegister, OtherProfile) -> OtherProfile_ExtendedWithNewHome" );
            
            Ok(other_gateway.register(
                ProfileId( Vec::from("OtherHome") ),
                mock::create_ownprofile( "Othereusz" ),
                None
            ))
        })
        .and_then(| otherprofile |{
            println!( "login() -> HomeSession" );

            own_gateway.login()
        })
        .and_then(| session |{
            println!( "ping(str) -> String" );
            
            session.ping( "dummy_ping" )
        })
        .and_then(| response |{
            println!( "{:?}" , response );
            println!( "login() -> HomeSession" );

            other_gateway.login()
        })
        .and_then(| othersession |{
            println!( "ping(str) -> String" );
            
            othersession.ping( "dummy_pong" )
        })
        .and_then(| otherresponse |{
            println!( "{:?}" , otherresponse );
            println!( "request pair() -> (gives back nothing or error)" );
            
            own_gateway.pair_request( "relation_dummy_type", "url" )
            
        })
        .and_then(|()|{

            println!( "request pair() -> (gives back nothing or error)" );
            
            other_gateway.pair_response( dummy_relation( "relation_dummy_type" ) )
            
        })
        .and_then(|()|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

            own_gateway.call(
                mock::dummy_relation( "work" ), 
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whatever" ) ) 
            )        
        })
        .and_then(|_|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

            other_gateway.call(
                mock::dummy_relation( "work" ), 
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whetavar" ) ) 
            )
        });

        
        println!( "All set up" );
        reactor.run( ownapp );
        
        println!( "We're done here, let's go packing" );
    }