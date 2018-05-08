#![allow(unused)]

extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate mercury_test;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;

// use ::net::*;
use mercury_test::dummy as dummy;
use mercury_test::dummy::*;

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
        println!( "***Setting up reactor and address variable" );
        let mut reactor = tokio_core::reactor::Core::new().unwrap();
        let handle = reactor.handle();

        let homeaddr = "/ip4/127.0.0.1/udp/9876";
        let homemultiaddr = homeaddr.to_multiaddr().unwrap();
        
        println!( "***Setting up signers" );

        let homesigno = Rc::new( dummy::Signo::new( "makusguba" ) );
        let other_homesigno = Rc::new( dummy::Signo::new( "tulfozotttea" ) );

        println!("***Setting up profiles");
        let homeprof = dummy::make_home_profile( &homeaddr ,homesigno.pub_key() );
        let other_homeprof = dummy::make_home_profile( &homeaddr ,other_homesigno.pub_key());
        
        println!("***ProfileGateway: ProfileSigner, DummyHome(as profile repo), HomeConnector" );

        let mut dht = ProfileStore::new();
        dht.insert(homeprof.id.clone(), homeprof.clone());
        dht.insert(other_homeprof.id.clone(), other_homeprof.clone());

        let mut home_storage = Rc::new( RefCell::new(dht) );
        let mut ownhomestore = Rc::clone(&home_storage);
        let mut home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , Rc::clone(&home_storage) ) ) );

        let other_signo = Rc::new( dummy::Signo::new( "Othereusz" ) );


        let signo = Rc::new( dummy::Signo::new( "Deuszkulcs" ) );
        let mut profile = make_own_persona_profile(signo.pub_key() );

        let own_gateway = ProfileGatewayImpl::new(
            signo,
            ownhomestore,
            Rc::new( dummy::DummyConnector::new_with_home( home ) ),
        );

        let sess = own_gateway.register(
                homesigno.prof_id().to_owned(),
                dummy::create_ownprofile( profile.clone() ),
                None
        )
        .map_err(|(p, e)|e)        
        .and_then(|session|{
            println!("user_one_requests");
            let f = other_signo.prof_id().0.clone();
            let problem = unsafe{String::from_utf8_unchecked(f)};
            own_gateway.pair_request( "relation_dummy_type", &problem )
        })
        .and_then(|own_profile|{
            println!( "user_one_login" );
            own_gateway.login()
        })
        .and_then(|session|{
            println!("user_one_events");
            session.events().take(1).collect()
            .map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing responded but something went wrong")))
        })
        .and_then(|pair_resp|{
            let resp_event = &pair_resp[0];
            println!("user_one_gets_response");
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
            println!("user_one_line_end");
            future::ok( call )
        });

        let mut other_home = Rc::new( RefCell::new( MyDummyHome::new( homeprof.clone() , Rc::clone(&home_storage) ) ) );
        let mut home_storage_other = Rc::clone(&home_storage);

        let mut other_profile = make_own_persona_profile(other_signo.pub_key() );
        let other_gateway = ProfileGatewayImpl::new(
            other_signo.clone(), 
            home_storage_other,
            Rc::new( dummy::DummyConnector::new_with_home( other_home ) ),
        );

        // let mut othersession : Box<HomeSession>;
        let other_reg = other_gateway.register(
            other_homesigno.prof_id().to_owned(),
            dummy::create_ownprofile( other_profile.clone() ),
            None
        )
        .map_err(|(p,e)|e)
        .and_then(| other_ownprofile |{
            println!("user_two_login");
            other_gateway.login()
        })
        .and_then(|other_session|{
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
            println!("user_two_events"); 
            let events = other_session.events();
            events.take(1).collect()
            .map_err(|_|ErrorToBeSpecified::TODO(String::from("pairing response.fail")))
            .and_then(|first|{
                println!("user_two_gets_request");
                let event = &first[0];
                match event{
                    &Ok(ProfileEvent::PairingRequest(ref half_proof))=>{
                        //TODO should look something like gateway.accept(half_proof)
                        Box::new(other_gateway.pair_response(
                                Relation::new(
                                &other_profile,
                                &RelationProof::from_halfproof(half_proof.clone(), other_gateway.signer.sign("apples".as_bytes())))
                        ))
                    },
                    _=>panic!("ProfileEvent assert fail")
                }
            })
            .and_then(move |_|{
                println!("user_two_checks_into_app");
                let other_chat = other_session.checkin_app( &ApplicationId( String::from( "SampleApp" ) ) );
                println!("user_two_line_ends");
                future::ok( other_chat )
            })
        });  

        let joined_f4t = Future::join(other_reg, sess); 
        let definitive_succes = reactor.run(joined_f4t);
        // let otherend = reactor.run( other_reg ).unwrap();
        // let ownend = reactor.run(sess).unwrap();
        // assert_eq!(ownend, ());
        // assert_eq!(otherend, ());
        println!( "***We're done here, let's go packing" );
    }


    //either user one cant send request because user two hasent registered yet, or user two cant respond because user one hasent sent a request