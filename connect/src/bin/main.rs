extern crate futures;
#[macro_use]
extern crate log;
extern crate log4rs;

#[macro_use]
extern crate clap;

extern crate mercury_connect;
extern crate mercury_home_protocol;
extern crate multiaddr;
//extern crate multihash;
extern crate tokio_core;


use clap::App;

use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use multiaddr::ToMultiaddr;

use futures::Future;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_protocol::crypto::*;
use mercury_connect::*;


fn main()
{
    log4rs::init_file( "log4rs.yml", Default::default() ).unwrap();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let client_private_key_file = matches.value_of("client-key-file").unwrap();
    let client_private_key = PrivateKey(std::fs::read(client_private_key_file).unwrap());
    let client_signer = Rc::new( Ed25519Signer::new(&client_private_key).unwrap() );
    let client_facet = ProfileFacet::Persona(PersonaFacet {homes: vec![], data: vec![]});
    let client_profile = Profile::new(&client_signer.profile_id(), &client_signer.public_key(), &client_facet);
    let client_own_profile = OwnProfile::new(&client_profile, &vec![]);

    // server details has to be taken from the command line
    // we need 3 pieces of information
    // (1) ProfileId
    // (2) Public key hash
    // (3) Address (since we don't yet have access to ipfs)
    let server_key_file = matches.value_of("server-key-file").unwrap();
    let srv_addr : SocketAddr = matches.value_of("server-addr").unwrap().parse().expect("Failed to parse server address");
    let addr = srv_addr.to_multiaddr().expect("Failed to parse server address");    

    let server_key = PublicKey(std::fs::read(server_key_file).unwrap());
    info!("homenode public key: {:?}", server_key);
    let server_id = ProfileId::from(&server_key);            
    info!("homenode profile id: {:?}", server_id);

    let mut reactor = reactor::Core::new().unwrap();
    let client_signer_clone = client_signer.clone();
    let client_signer_clone2 = client_signer.clone();

    let mut profile_store = SimpleProfileRepo::new();

    let home_profile = Profile::new_home(server_id.clone(), server_key, addr);
    profile_store.insert(home_profile);

    let home_connector = SimpleTcpHomeConnector::new(reactor.handle());
    let profile_gw = ProfileGatewayImpl::new(client_signer_clone, Rc::new(profile_store),  Rc::new(home_connector));
    let test_fut = profile_gw.connect_home(&server_id.clone())
        .and_then(|home| {
            let halfproof = RelationHalfProof::new("home", &server_id, &*client_signer);
            home.register(client_own_profile, halfproof, None)
                .map(|_own_profile| home)
                .map_err( |(_own_profile, e)| e )

        })            
        .and_then(move |home| {
            info!("connected, logging in");
            home.login(client_signer_clone2.profile_id())

        })
        .and_then(|session| {
            info!("session created, sending ping");
            session.ping("hahoooo")
        })
        .map(|pong| {
            info!("received pong");
            pong
        })
        .map_err(|err| {
            warn!("error: {:?}", err);
            ErrorToBeSpecified::TODO(String::from("profile gateway failed to login()"))
        });

    let pong = reactor.run(test_fut);
    
    debug!("Response: {:?}", pong);

    let handle = reactor.handle();
    let result = reactor.run( reactor::Timeout::new( Duration::from_secs(5), &handle ).unwrap() );
    info!("Client result {:?}", result);
}