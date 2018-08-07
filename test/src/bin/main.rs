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
use mercury_test::dummy::{self, *};

use mercury_connect::*;
use mercury_home_protocol::*;
use mercury_test::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::iter::{Iterator};
use std::io::{BufRead, Read, Write, stdin};

use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

use futures::{future, Future, Stream};



fn main(){
    //print!("{}[2J", 27 as char);
    println!("Setting up config\n");
    let mut reactor = reactor::Core::new().unwrap();
    let reactorhandle = reactor.handle();
    let homeaddr = "/ip4/127.0.0.1/udp/9876";
    let homemultiaddr = homeaddr.to_multiaddr().unwrap();
    
    let (profile, signo) = generate_profile(ProfileFacet::Persona(PersonaFacet{homes: vec![], data: vec![]}));
    let (homeprof, homesigno) = generate_profile(ProfileFacet::Home(HomeFacet{addrs: vec![homemultiaddr.clone().into()], data: vec![]}));
    
    println!("Setting up connection\n");

    let mut dht = ProfileStore::new();
    dht.insert(homeprof.id.clone(), homeprof.clone());
    let mut home_storage = Rc::new(dht);
    let mut store_rc = Rc::clone(&home_storage);
    let mut home = Rc::new( MyDummyHome::new( homeprof.clone() , home_storage ) );

    let signo = Rc::new(signo);
    let profilegateway = ProfileGatewayImpl{
        signer:         signo,
        profile_repo:   store_rc,
        home_connector: Rc::new( dummy::DummyConnector::new_with_home( home ) ),
    };

    println!("\nRegistering\n");
    let reg = profilegateway.register(homesigno.profile_id().to_owned(), dummy::create_ownprofile( profile ), None);
    let ownprofile = reactor.run(reg).unwrap();
    
    println!("\nLogging in\n");

    let session = reactor.run( profilegateway.login() ).unwrap();
    
    println!("\nAll set up\n");
    
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
            // "2" =>{
            //     profilegateway.call(
            //         dummy::dummy_relation("work"), 
            //         ApplicationId( String::from("SampleApp") ), 
            //         AppMessageFrame("whatever".as_bytes().to_owned() ),
            //         None
            //     );
    
            // }
            "3" =>{
                profilegateway.pair_request("relation_dummy_type", &ProfileId(b"profile_id".to_vec()), None);
    
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


//Call handling test code

/*       
        let (sen, rec) : (mpsc::Sender<Result<AppMessageFrame, String>>, mpsc::Receiver<Result<AppMessageFrame, String>>) = mpsc::channel(1);

        let incoming = CallRequest{
            relation:       dummy_relation_proof("whatever"),
            init_payload:   AppMessageFrame(Vec::from("internal shit")),
            to_caller:      Some(sen),
        };
        let incall_impl = Incall{request : incoming};
        let ptr = incall_impl.request();
        
        let sink = ptr.to_caller.to_owned().unwrap();
        reactor.run(sink.send(Ok(AppMessageFrame(Vec::from("sink.send")))));
        println!("\n {:?}", AppMessageFrame(Vec::from("sink.send")));
        let receive_fut = rec.take(1).collect();
        let received_msg = reactor.run(receive_fut);
        println!("\n {:?}", received_msg);*/