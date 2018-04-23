#[allow(unused)]
#[cfg(test)]
use super::*;

extern crate mercury_home_protocol;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;

use std::cell::RefCell;
use std::rc::Rc;
use std::io::{BufRead, Read, Write, stdin};

use mercury_home_protocol::*;

use ::dummy::*;
use ::net::*;
use ::protocol_capnp::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};
use futures::{Future,Stream};

    #[test]
    fn test_register(){

        let mut setup = dummy::TestSetup::setup();

        let ownprofile = setup.profilegate.register(
                setup.homeprofileid,
                setup.userownprofile,
                None
        );

        let res = setup.reactor.run(ownprofile);      
    }

    #[test]
    fn test_unregister(){

        let mut setup = dummy::TestSetup::setup();

        let registered = setup.profilegate.register(
                setup.homeprofileid.clone(),
                setup.userownprofile,
                None
        );
        let res = setup.reactor.run(registered); 
        //assert if registered

        let unregistered = setup.profilegate.unregister(
                setup.homeprofileid,
                setup.userid,
                None
        );

        let res = setup.reactor.run(unregistered);     

        //assert if unregistered 
    }

    #[test]
    fn test_login(){

        let mut setup = dummy::TestSetup::setup();

        let home_session = setup.profilegate.login();

        let res = setup.reactor.run(home_session);      
    }

    #[test]
    fn test_claim(){

        let mut setup = dummy::TestSetup::setup();

        let home_session = setup.profilegate.claim(
                setup.homeprofileid,
                setup.userid,
        );

        let res = setup.reactor.run(home_session);      
    }
    
    #[test]
    fn test_update(){

        let mut setup = dummy::TestSetup::setup();
        let other_home_signer = Signo::new("otherhome");
        let otherhome = make_home_profile("/ip4/127.0.0.1/udp/9876", other_home_signer.pub_key());

        setup.home.borrow_mut().insert(otherhome.id.clone(), otherhome.clone());
        let home_session = setup.profilegate.update(
            otherhome.id,
            &setup.userownprofile,
        );

        let res = setup.reactor.run(home_session);      
    }

    #[test]
    fn test_call(){

        let mut setup = dummy::TestSetup::setup();

        let call_messages = setup.profilegate.call(
            dummy::dummy_relation("test_relation"),
            ApplicationId( String::from( "Undertale" ) ), 
            AppMessageFrame( Vec::from( "Megalovania" ) ),
            None
        );

        let res = setup.reactor.run(call_messages);      
    }

    #[test]
    fn test_ping(){

        let mut setup = dummy::TestSetup::setup();

        let response = setup.profilegate.login()
        .and_then(|home_session|{
            home_session.ping( "test_ping" )
        });

        let res = setup.reactor.run(response);      
    }

    #[test]
    fn test_pair_req(){

        let signo = Rc::new( dummy::Signo::new( "TestKey" ) );
        let mut setup = dummy::TestSetup::setup();

        let zero = setup.profilegate.pair_request( "test_relation", "test_url" );

        let res = setup.reactor.run(zero);   
    }

    #[test]
    fn test_pair_res(){

        let mut setup = dummy::TestSetup::setup();
        let zero = setup.profilegate.pair_response(
                dummy::dummy_relation("test_relation"));

        let res = setup.reactor.run(zero);      
    }

    #[test]
    fn test_relations(){

        let mut setup = dummy::TestSetup::setup();

        let zero = setup.profilegate.relations( &setup.userid );

        let res = setup.reactor.run(zero);
    }

    #[test]
    fn and_then_story(){
        //print!("{}[2J", 27 as char);
        println!( "Setting up config" );
        let mut reactor = tokio_core::reactor::Core::new().unwrap();
        let handle = reactor.handle();

        let homeaddr = "/ip4/127.0.0.1/udp/9876";
        let homemultiaddr = homeaddr.to_multiaddr().unwrap();
        
        println!( "Setting up signers" );
        let signo = Rc::new( dummy::Signo::new( "Deuszkulcs" ) );
        let other_signo = Rc::new( dummy::Signo::new( "Othereusz" ) );
        let homesigno = Rc::new( dummy::Signo::new( "makusguba" ) );
        let other_homesigno = Rc::new( dummy::Signo::new( "tulfozotttea" ) );

        println!("Setting up profiles");
        let homeprof = dummy::make_home_profile( &homeaddr ,homesigno.pub_key() );
        let other_homeprof = dummy::make_home_profile( &homeaddr ,other_homesigno.pub_key() );

        let mut profile = make_own_persona_profile(signo.pub_key() );
        let mut other_profile = make_own_persona_profile(other_signo.pub_key() );

        println!( "ProfileGateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );

        let mut dht = ProfileStore::new();
        dht.insert(homeprof.id.clone(), homeprof.clone());
        dht.insert(other_homeprof.id.clone(), other_homeprof.clone());

        let mut home_storage = Rc::new( RefCell::new(dht) );
        let mut home_storage_other = Rc::clone(&home_storage);
        let mut store_rc = Rc::clone(&home_storage);
        let mut store_rc_other = Rc::clone(&home_storage);
        let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , home_storage ) ) );
        let mut other_home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , home_storage_other ) ) );

        let own_gateway = ProfileGatewayImpl::new(
            signo,
            store_rc,
            Rc::new( dummy::DummyConnector::new_with_home( home ) ),
        );

        let other_gateway = ProfileGatewayImpl::new(
            other_signo,
            store_rc_other,
            Rc::new( dummy::DummyConnector::new_with_home( other_home ) ),
        );

        println!( "any_home_of(profile) -> Home" );
        let ownapp = own_gateway.any_home_of(&profile)
        .and_then(|home|{
            println!( "register(HomeProfile_Id_WhereWeRegister, OwnProfile) -> OwnProfile_ExtendedWithNewHome" );
            
            let deusz = dummy::make_own_persona_profile(&PublicKey( Vec::from( "pubkey" ) ) );
            Ok(own_gateway.register(
                ProfileId( Vec::from("Home") ),
                dummy::create_ownprofile( deusz ),
                None
            ))
        })
        .and_then(|ownprofile|{
            other_gateway.any_home_of(&profile)
        })
        .and_then(| otherhome |{
            println!( "register(HomeProfile_Id_WhereWeRegister, OtherProfile) -> OtherProfile_ExtendedWithNewHome" );

            let persona = dummy::make_own_persona_profile(&PublicKey( Vec::from( "pubkey" ) ) );
            Ok(other_gateway.register(
                ProfileId( Vec::from("OtherHome") ),
                dummy::create_ownprofile( persona ),
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
            
            other_gateway.pair_response(
                dummy::dummy_relation("test_relation"),
            )
        })
        .and_then(|()|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

            own_gateway.call(
                dummy::dummy_relation("test_relation"),
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whatever" ) ),
                None
            )        
        })
        .and_then(|_|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

            other_gateway.call(
                dummy::dummy_relation("test_relation"), 
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whetavar" ) ),
                None
            )
        });

        
        println!( "All set up" );
        reactor.run( ownapp );
        
        println!( "We're done here, let's go packing" );
    }