use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use clap::{load_yaml, App};
use failure::Fail;
use futures::prelude::*;
use log::*;
use multiaddr::ToMultiaddr;
use tokio_core::reactor;

use keyvault::PublicKey as KeyVaultPublicKey;
use mercury_connect::profile::MyProfileImpl;
use mercury_connect::*;
use mercury_home_protocol::*;

fn main() {
    log4rs::init_file("log4rs.yml", Default::default()).unwrap();
    // TODO use structopt instead of yaml+clap here
    let yaml = load_yaml!("protocol-sample-cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let client_private_key_file = matches.value_of("client-key-file").unwrap();
    let client_private_key_bytes = std::fs::read(client_private_key_file).unwrap();
    let client_private_key_ed =
        ed25519::EdPrivateKey::from_bytes(client_private_key_bytes).unwrap();
    let client_private_key = PrivateKey::from(client_private_key_ed);
    let client_signer = Rc::new(crypto::PrivateKeySigner::new(client_private_key).unwrap());
    //let client_facet = ProfileFacet::Persona(PersonaFacet { homes: vec![], data: vec![] });
    let client_profile = Profile::new(&client_signer.public_key()); //, &client_facet);
    let client_own_profile = OwnProfile::new(&client_profile, &vec![]);

    // server details has to be taken from the command line
    // we need 3 pieces of information
    // (1) ProfileId
    // (2) Public key hash
    // (3) Address (since we don't yet have access to ipfs)
    let server_key_file = matches.value_of("server-key-file").unwrap();
    let srv_addr: SocketAddr =
        matches.value_of("server-addr").unwrap().parse().expect("Failed to parse server address");
    let addr = srv_addr.to_multiaddr().expect("Failed to parse server address");

    let server_key_bytes = std::fs::read(server_key_file).unwrap();
    let server_key_ed = ed25519::EdPublicKey::from_bytes(server_key_bytes).unwrap();
    let server_key = PublicKey::from(server_key_ed);
    info!("homenode public key: {:?}", server_key);
    let server_id = server_key.key_id();
    info!("homenode profile id: {:?}", server_id);
    let home_attrs = HomeFacet::new(vec![addr], vec![]).to_attributes();
    let home_profile = Profile::create(server_key, 1, vec![], home_attrs);

    let profile_store = SimpleProfileRepo::default();
    profile_store.insert(home_profile);

    let mut reactor = reactor::Core::new().unwrap();
    let home_connector = SimpleTcpHomeConnector::new(reactor.handle());
    let profile_gw = MyProfileImpl::new(
        client_own_profile.clone(),
        client_signer.clone(),
        Rc::new(profile_store),
        Rc::new(home_connector),
        reactor.handle(),
    );
    let test_fut = profile_gw
        .connect_home(&server_id.clone())
        .map_err(|err| {
            err.context(mercury_home_protocol::error::ErrorKind::ConnectionToHomeFailed).into()
        })
        .and_then(|home| {
            info!("connected, registering");
            let halfproof = RelationHalfProof::new(
                RelationProof::RELATION_TYPE_HOSTED_ON_HOME,
                &server_id,
                &*client_signer,
            );
            home.register(client_own_profile, halfproof)
                .map(|own_profile| (own_profile, home))
                .map_err(|(_own_profile, e)| e)
        })
        .and_then(move |(own_profile, home)| {
            info!("registered, logging in");
            let home_proof = match own_profile.profile.as_persona() {
                Some(ref persona) => persona.homes.get(0).map(|item| item.to_owned()),
                None => None,
            };
            match home_proof {
                Some(proof) => home.login(&proof),
                None => Box::new(
                    Err(mercury_home_protocol::error::ErrorKind::LoginFailed.into()).into_future(),
                ),
            }
        })
        .and_then(|session| {
            info!("session created, sending ping");
            session.ping("hahoooo")
        })
        .map(|pong| {
            info!("received pong");
            pong
        });

    let pong = reactor.run(test_fut);

    debug!("Response: {:?}", pong);

    let timeout = reactor::Timeout::new(Duration::from_secs(5), &reactor.handle()).unwrap();
    let result = reactor.run(timeout);
    info!("Client result {:?}", result);
}
