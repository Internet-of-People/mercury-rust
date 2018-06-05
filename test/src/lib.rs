extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate mercury_home_node;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_stdin_stdout;
extern crate multiaddr;
extern crate multihash;

pub mod dummy;

#[cfg(test)]
mod test{

use super::*;
use ::dummy::*;

use std::net::ToSocketAddrs;
use std::rc::Rc;
use std::cell::RefCell;
use std::io::{BufRead, Read, Write, stdin};

use multihash::{encode, Hash};
use multiaddr::{ToMultiaddr, Multiaddr};

use futures::future;
use futures::{Future, Stream};
use tokio_core::net::{TcpListener, TcpStream};
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};

use mercury_home_protocol::*;
use mercury_connect::*;
use ::dummy::{ MyDummyHome, Signo, make_home_profile, ProfileStore, };
use mercury_connect::protocol_capnp::HomeClientCapnProto;
use mercury_home_node::protocol_capnp::HomeDispatcherCapnProto;

use mercury_connect::ProfileGateway;
use mercury_connect::ProfileGatewayImpl;

#[test]
fn test_events()
{
    let mut reactor = reactor::Core::new().unwrap();

    let homeaddr = "127.0.0.1:9876";
    let addr = homeaddr.clone().to_socket_addrs().unwrap().next().expect("Failed to parse address");

    let homemultiaddr = "/ip4/127.0.0.1/udp/9876".to_multiaddr().unwrap();
    let homesigno = Rc::new(Signo::new("makusguba"));
    let homeprof = Profile::new_home(homesigno.prof_id().to_owned(), homesigno.pub_key().to_owned(), homemultiaddr.clone());

    let mut dht = ProfileStore::new();
    dht.insert(homeprof.id.clone(), homeprof.clone());
    let mut home_storage = Rc::new( RefCell::new(dht) );

    let handle1 = reactor.handle();
    let server_socket = TcpListener::bind( &addr, &reactor.handle() ).expect("Failed to bind socket");
    let server_fut = server_socket.incoming().for_each( move |(socket, addr)|
    {
        println!("Accepted client connection, serving requests");
        //let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , home_storage ) ) );
        let mut store_clone = Rc::clone(&home_storage);
        let home = Box::new( MyDummyHome::new( homeprof.clone() , store_clone ) );
        HomeDispatcherCapnProto::dispatch_tcp( home, socket, handle1.clone() );
        Ok( () )
    } );

    let handle2 = reactor.handle();
    let client_fut = TcpStream::connect( &addr, &reactor.handle() )
        .map_err( |e| ErrorToBeSpecified::TODO(String::from("test_events fails at connect ")))
        .and_then( |tcp_stream|
        {
            let signer = Rc::new( Signo::new("privatekey") );
            let my_profile = signer.prof_id().clone();
            let home_profile = make_home_profile("localhost:9876", signer.pub_key());
            let home_ctx = Box::new( HomeContext::new(signer, &home_profile) );
            let client = HomeClientCapnProto::new_tcp( tcp_stream, home_ctx, handle2 );
            client.login(my_profile) // TODO maybe we should require only a reference in login()
        } )
        .map( |session|
        {
            session.events() //.for_each( |event| () )
        } );

//    let futs = server_fut.select(client_fut);
//    let both_fut = select_ok( futs.iter() ); // **i as &Future<Item=(),Error=()> ) );
//    let result = reactor.run(both_fut);
}

#[test]
    fn test_register(){

        let mut setup = dummy::TestSetup::setup();

        let mut registered_ownprofile = setup.userownprofile.clone();
        let relation_proof = RelationProof::new(
            "home", 
            &registered_ownprofile.profile.id, 
            &Signature(registered_ownprofile.profile.pub_key.0.clone()), 
            &setup.homeprofile.id, 
            &Signature(setup.homeprofile.pub_key.0.clone())
        );
        
        match registered_ownprofile.profile.facets[0]{
            ProfileFacet::Persona(ref mut facet)=>{
                facet.homes.push(relation_proof);
            },
            _=>{
                panic!("test_register failed cause Deusz fucked up");
            }
        }

        let ownprofile = setup.profilegate.register(
                setup.homeprofileid,
                setup.userownprofile,
                None
        );

        let res = setup.reactor.run(ownprofile).unwrap();
   
        assert_eq!(res, registered_ownprofile);  
    }

    #[test]
    fn test_unregister(){
        let mut setup = dummy::TestSetup::setup();

        let homeless_profile = setup.userownprofile.clone();
        let homeid = setup.homeprofileid.clone();
        let userid = setup.userid.clone();
        let mut registered = setup.profilegate.register(
                setup.homeprofileid.clone(),
                setup.userownprofile.clone(),
                None
        );
        let reg = setup.reactor.run(registered).unwrap();
        //see test_register() to see if registering works as intended
        let unreg = setup.profilegate.unregister(
            homeid,
            userid,
            None
        );
        let res = setup.reactor.run(unreg).unwrap(); 
        //TODO needs HomeSession unregister implementation    
        //assert_eq!(res, homeless_profile);
    }

    #[test]
    fn test_login(){

        let mut setup = dummy::TestSetup::setup();

        let home_session = setup.profilegate.login();

        let res = setup.reactor.run(home_session);      
    }

    #[test]
    fn test_ping(){
        //TODO ping function only present for testing phase, incorporate into test_login?
        let mut setup = dummy::TestSetup::setup();

        let response = setup.profilegate.login()
        .and_then(|home_session|{
            home_session.ping( "test_ping" )
        });

        let res = setup.reactor.run(response);      
    }

    #[test]
    fn test_claim(){
        //profile registering is required
        let mut setup = dummy::TestSetup::setup();

        let home_session = setup.profilegate.claim(
                setup.homeprofileid,
                setup.userid,
        );

        let res = setup.reactor.run(home_session).unwrap();
        //TODO needs home.claim implementation
        println!("Claimed : {:?} ||| Stored : {:?}", res, setup.userownprofile);        
        assert_eq!(res, setup.userownprofile);      
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
        //TODO needs homesession.update implementation
        //session updates profile stored on home(?)
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
        //TODO needs home.call implementation...
        let res = setup.reactor.run(call_messages);      
    }

    #[test]
    fn test_pair_req(){
        //TODO could be tested by sending pair request and asserting the events half_proof that the peer receives to what is should be
        let signo = Rc::new( dummy::Signo::new( "TestKey" ) );
        let mut setup = dummy::TestSetup::setup();

        let zero = setup.profilegate.pair_request( "test_relation", "test_url" );

        let res = setup.reactor.run(zero);   
    }

    #[test]
    fn test_pair_res(){
        //TODO could be tested by sending pair response and asserting the events relation_proof that the peer receives to what is should be
        let mut setup = dummy::TestSetup::setup();
        let zero = setup.profilegate.pair_response(
                dummy::dummy_relation("test_relation"));

        let res = setup.reactor.run(zero);      
    }

    #[test]
    fn test_relations(){
        //TODO test by storing relations and asserting the return value of relations to those that were stored
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

        let sess = own_gateway.register(
                homesigno.prof_id().to_owned(),
                dummy::create_ownprofile( profile.clone() ),
                None
        )
        .map_err(|(p, e)|e)
        .and_then(|_|{
            println!( "login() -> HomeSession" );
            own_gateway.login()
        });

        let session = reactor.run(sess).unwrap();

        let req = own_gateway.pair_request( "relation_dummy_type", "url" )
        .and_then(|_|{
            // other_session.events().for_each(|event|{
            //     match event{
            //         Ok(ProfileEvent::PairingRequest(half_proof))=>{
            //             Box::new(other_gateway.pair_response(
            //                 Relation::new(
            //                     &other_profile,
            //                     &RelationProof::from_halfproof(half_proof.clone(), other_gateway.signer.sign(&[111,123,143])))
            //             ).map_err(|_|())) as Box<Future<Item=(),Error = ()> >
            //         },
            //         _=>Box::new(future::ok(()))
            //     }
            // }).map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing response.fail")))

            other_session.events().take(1).collect()
            .map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing response.fail")))
            

        })
        .and_then(|first|{
            println!( " pairing_response() -> (gives back nothing or error)" );
            let event = &first[0];
            match event{
                &Ok(ProfileEvent::PairingRequest(ref half_proof))=>{
                    //TODO should look something like gateway.accept(half_proof)
                    Box::new(other_gateway.pair_response(
                            Relation::new(
                            &other_profile,
                            &RelationProof::from_halfproof(half_proof.clone(), other_gateway.signer.sign(&[111,123,143])))
                    ))
                },
                _=>panic!("ProfileEvent assert fail")
            }
        })
        .and_then(|_|{

            session.events().take(1).collect()
            .map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing responded but something went wrong")))
            

        })
        .and_then(|pair_resp|{
            let resp_event = &pair_resp[0];
            match resp_event{
                &Ok(ProfileEvent::PairingResponse(ref relation_proof))=>{
                    println!("{:?}", relation_proof);
                    future::ok(relation_proof.clone())
                },
                _=>panic!("ProfileEvent assert fail")
            }
        })
        .and_then(|relation_proof|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );
            let relation = Relation::new(&profile,&relation_proof);
            future::ok(own_gateway.call(
                relation,
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whatever" ) ),
                None
            ))
        })
        .and_then(|_|{
            println!( "call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );

            future::ok(other_session.checkin_app(&ApplicationId( String::from( "SampleApp" ) )))
        });

        
        println!( "All set up" );
        reactor.run( req );
        
        println!( "We're done here, let's go packing" );
    }

     #[test]
    fn name() {
        unimplemented!();
    }
}