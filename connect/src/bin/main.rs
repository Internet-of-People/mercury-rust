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

use futures::{future, Future, Stream};

// fn main(){
//     //print!("{}[2J", 27 as char);
//     println!("Setting up config\n");
//     let mut reactor = reactor::Core::new().unwrap();
//     let reactorhandle = reactor.handle();
//     let homeaddr = "/ip4/127.0.0.1/udp/9876";
//     let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    
//     println!("Setting up signers\n");
//     let signo = Rc::new(dummy::Signo::new("Deusz"));
//     let homesigno = Rc::new(dummy::Signo::new("Home"));
    
//     println!("Setting up home\n");

//     let homeprof = Profile::new_home(homesigno.prof_id().to_owned(), homesigno.pub_key().to_owned(), homemultiaddr.clone());
//     let profile = make_own_persona_profile(signo.pub_key());
    
//     println!("Setting up connection\n");

//     let mut dht = ProfileStore::new();
//     dht.insert(homeprof.id.clone(), homeprof.clone());
//     let mut home_storage = Rc::new( RefCell::new(dht) );
//     let mut store_rc = Rc::clone(&home_storage);
//     let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , home_storage ) ) );

//     let profilegateway = ProfileGatewayImpl{
//         signer:         signo,
//         profile_repo:   store_rc,
//         home_connector: Rc::new( dummy::DummyConnector::new_with_home( home ) ),
//     };

//     println!("\nRegistering\n");
//     let reg = profilegateway.register(homesigno.prof_id().to_owned(), dummy::create_ownprofile( profile ), None);
//     let ownprofile = reactor.run(reg).unwrap();
    
//     println!("\nLogging in\n");

//     let session = reactor.run( profilegateway.login() ).unwrap();
    
//     println!("\nAll set up\n");
    
//     println!("Menu\n1. Connect\n2. Call(crashes)\n3. Pair\n4. Ping\n5. Show profile\nExit with ctrl+d");
//     let stdin = tokio_stdin_stdout::stdin(1);
//     let bufreader = std::io::BufReader::new(stdin);
//     let instream = tokio_io::io::lines(bufreader);
//     let stdin_closed = instream.for_each(|line|{     
//         match line.as_ref(){
//             "1" =>{
//                 let signer = profilegateway.signer.to_owned();
//                 profilegateway.home_connector.connect(&homeprof, signer);
//                 println!("connect");
    
//             },
//             // "2" =>{
//             //     profilegateway.call(
//             //         dummy::dummy_relation("work"), 
//             //         ApplicationId( String::from("SampleApp") ), 
//             //         AppMessageFrame("whatever".as_bytes().to_owned() ),
//             //         None
//             //     );
    
//             // }
//             "3" =>{
//                 profilegateway.pair_request("relation_dummy_type", "url");
    
//             }
//             "4" =>{
//                 session.ping("dummy_ping");
    
//             }
//             "5" =>{
//                 println!("{:?}", ownprofile);
    
//             }
//             _ =>{
//                 println!("nope");
    
//             },
//         };
//         futures::future::ok::<(),std::io::Error>(())
//     });
//     let crash = reactor.run(stdin_closed).unwrap();
// }


    fn main(){
        //print!("{}[2J", 27 as char);
        println!( "***Setting up config" );
        let mut reactor = tokio_core::reactor::Core::new().unwrap();
        let handle = reactor.handle();

        let homeaddr = "/ip4/127.0.0.1/udp/9876";
        let homemultiaddr = homeaddr.to_multiaddr().unwrap();
        
        println!( "***Setting up signers" );

        let homesigno = Rc::new( dummy::Signo::new( "makusguba" ) );
        let other_homesigno = Rc::new( dummy::Signo::new( "tulfozotttea" ) );

        println!("***Setting up profiles");
        let homeprof = dummy::make_home_profile( &homeaddr ,homesigno.pub_key() );
        let other_homeprof = dummy::make_home_profile( &homeaddr ,other_homesigno.pub_key() );
        
        println!("***ProfileGateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );

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
            println!( "login() -> HomeSession" );

            other_gateway.login()
        });
        let other_session = reactor.run(other_reg).unwrap();
        println!("***registered callee profile");
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

        println!("***Sending pairing request");
        let req = own_gateway.pair_request( "relation_dummy_type", &other_gateway.get_base64_id() )
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
            println!( " ***pairing_response() -> (gives back nothing or error)" );
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
            println!( "***call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );
            let relation = Relation::new(&profile,&relation_proof);
            let call = own_gateway.call(
                relation,
                ApplicationId( String::from( "SampleApp" ) ), 
                AppMessageFrame( Vec::from( "whatever" ) ),
                None
            );
            future::ok( call )

        })
        .and_then(|_|{
            println!( "***call(RelationWithCallee, InWhatApp, InitMessage) -> CallMessages" );
            let other_chat = other_session.checkin_app( &ApplicationId( String::from( "SampleApp" ) ) );
            println!("other chat : {:?}", other_chat);
            future::ok( other_chat )
        });
        println!( "***All set up" );
        reactor.run( req );
        
        println!( "***We're done here, let's go packing" );
    }