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

        let homesigno = Rc::new( dummy::Signo::new( "makusguba" ) );
        let other_homesigno = Rc::new( dummy::Signo::new( "tulfozotttea" ) );

        println!("Setting up profiles");
        let homeprof = dummy::make_home_profile( &homeaddr ,homesigno.pub_key() );
        let other_homeprof = dummy::make_home_profile( &homeaddr ,other_homesigno.pub_key() );
        
        println!( "ProfileGateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );

        let mut dht = ProfileStore::new();
        dht.insert(homeprof.id.clone(), homeprof.clone());
        dht.insert(other_homeprof.id.clone(), other_homeprof.clone());

        let mut home_storage = Rc::new( RefCell::new(dht) );
        let mut home_storage_other = Rc::clone(&home_storage);

        let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , Rc::clone(&home_storage) ) ) );
        let mut other_home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , Rc::clone(&home_storage) ) ) );

        let other_signo = Rc::new( dummy::Signo::new( "Othereusz" ) );
        let mut other_profile = make_own_persona_profile(other_signo.pub_key() );

        let other_gateway = ProfileGatewayImpl::new(
            other_signo, 
            home_storage_other,
            Rc::new( dummy::DummyConnector::new_with_home( other_home ) ),
        );

        let other_reg = other_gateway.register(
            other_homesigno.prof_id().to_owned(),
            dummy::create_ownprofile( other_profile.clone() ),
            None
        )
        .map_err(|(p,e)|e)
        .and_then(| response |{
            println!( "{:?}" , response );
            println!( "login() -> HomeSession" );

            other_gateway.login()
        });
        let other_session = reactor.run(other_reg).unwrap();
        println!("registered callee profile");

        let signo = Rc::new( dummy::Signo::new( "Deuszkulcs" ) );
        let mut profile = make_own_persona_profile(signo.pub_key() );

        let own_gateway = ProfileGatewayImpl::new(
            signo,
            home_storage,
            Rc::new( dummy::DummyConnector::new_with_home( home ) ),
        );

        let reg = own_gateway.register(
                homesigno.prof_id().to_owned(),
                dummy::create_ownprofile( profile ),
                None
        )
        .map_err(|(p, e)|e)
        .and_then(|_|{
            println!( "login() -> HomeSession" );
            own_gateway.login()
        })
        // .and_then(| session |{
        //     println!( "ping(str) -> String" );
            
        //     session.ping( "dummy_ping" )
        // })
        
        // .and_then(| othersession |{
        //     println!( "ping(str) -> String" );
            
        //     othersession.ping( "dummy_pong" )
        // })
        .and_then(| session |{
            // println!( "{:?}" , otherresponse );
            println!( "request pair() -> (gives back nothing or error)" );
            
            own_gateway.pair_request( "relation_dummy_type", "url" )
            
        })
        .and_then(|_|{
            other_session.events().for_each(|event|{
                match event{
                    Ok(ProfileEvent::PairingRequest(half_proof))=>{
                        Box::new(other_gateway.pair_response(
                            Relation::new(
                                &other_profile,
                                &RelationProof::from_halfproof(half_proof.clone(), other_gateway.signer.sign(&[111,123,143])))
                        ).map_err(|_|())) as Box<Future<Item=(),Error = ()> >
                    },
                    _=>Box::new(future::ok(()))
                }
            }).map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing response.fail")))
        })
        .and_then(|_|{

            println!( "request pair() -> (gives back nothing or error)" );
            
            other_gateway.pair_response(
                dummy::dummy_relation("test_relation"),
            )
        })
        .and_then(|_|{
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
        reactor.run( reg );
        
        println!( "We're done here, let's go packing" );
    }