// NOTE that this file contains initialization code for the Mercury Connect Service
//      that will not be part of this program. Instead, it will run in a separated,
//      protected background service and communicate with dApps through IPC.
//      However, until it's properly implemented, dApps have to contain and instantiate it.

use std::time::Duration;

use super::*;
use mercury_connect::profile::EventStream;
use multiaddr::{Multiaddr, ToMultiaddr};



pub fn init_connect_service(my_private_profilekey_file: &str, home_id_str: &str,
                            home_addr_str: &str, reactor: &mut reactor::Core)
    -> Result<(Rc<ConnectService>, ProfileId, ProfileId), std::io::Error>
{
    use mercury_connect::service::{DummyUserInterface, MyProfileFactory, SignerFactory};
    use mercury_storage::async::{KeyAdapter, KeyValueStore, fs::FileStore, imp::InMemoryStore};

    debug!("Initializing service instance");

    let home_pubkey = PublicKey(std::fs::read(home_id_str)?);
    let home_id = ProfileId::from(&home_pubkey);
    let home_addr :SocketAddr = home_addr_str.parse().map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    let home_multiaddr : Multiaddr = home_addr.to_multiaddr().expect("Failed to parse server address");
    let home_profile = Profile::new_home( home_id.clone(), home_pubkey.clone(), home_multiaddr );

    let my_private_key = PrivateKey(std::fs::read(my_private_profilekey_file)?);
    let my_signer = Rc::new( Ed25519Signer::new(&my_private_key).unwrap() ) as Rc<Signer>;
    let my_profile_id = my_signer.profile_id().to_owned();
    let my_profile = Profile::new( &my_profile_id, my_signer.public_key(),
        &ProfileFacet::Persona( PersonaFacet{homes: vec![], data: vec![]} ) );

    // TODO consider that client should be able to start up without being a DHT client,
    //      e.g. with having only a Home URL including hints to access Home
    let profile_repo = SimpleProfileRepo::from( KeyAdapter::<String,_,_>::new(
        FileStore::new("/tmp/mercury/thebutton-storage").unwrap() ) );
//    let profile_repo = SimpleProfileRepo::default();
    let repo_initialized = reactor.run( profile_repo.load(&my_profile_id) );
    if repo_initialized.is_err()
    {
        debug!("Profile repository was not initialized, populate it with required entries");
        reactor.run( profile_repo.insert(home_profile) ).unwrap();
        reactor.run( profile_repo.insert(my_profile.clone() ) ).unwrap();
    }
    else { debug!("Profile repository was initialized, continue without populating it"); }
    let profile_repo = Rc::new(profile_repo);

    let my_profiles = Rc::new( vec![ my_profile_id.clone() ].iter().cloned().collect::<HashSet<_>>() );
    let my_own_profile = OwnProfile::new(&my_profile,&[]);
    let signers = vec![ ( my_profile_id.clone(), my_signer ) ].into_iter().collect();
    let signer_factory: Rc<SignerFactory> = Rc::new(SignerFactory::new(signers) );
    let home_connector = Rc::new( SimpleTcpHomeConnector::new( reactor.handle() ) );
    let gateways = Rc::new( MyProfileFactory::new(
        signer_factory, profile_repo.clone(), home_connector, reactor.handle() ) );

    let ui = Rc::new( DummyUserInterface::new( my_profiles.clone() ) );
    let mut own_profile_store = InMemoryStore::new();
    reactor.run( own_profile_store.set(my_profile_id.clone(), my_own_profile ) ).unwrap();
    let profile_store = Rc::new( RefCell::new(own_profile_store) );
    let service = Rc::new( ConnectService::new(ui, my_profiles, profile_store, gateways) ); //, &reactor.handle() ) );

    Ok( (service, my_profile_id, home_id) )
}



pub fn init_app_common(app_context: &AppContext)
    -> Box< Future<Item=Rc<MyProfile>, Error=std::io::Error> >
{
    let client_id = app_context.client_id.clone();
    let home_id = app_context.home_id.clone();
    let init_fut = app_context.service.admin_session(None)
        .inspect( |_admin| debug!("Admin endpoint was connected") )
        .and_then( move |admin| admin.profile(client_id) )
        .and_then( move |my_profile| my_profile.join_home(home_id, None )
            .map( |()| my_profile )
            .map_err( |e| {
                debug!("Failed to join home: {:?}", e);
                mercury_connect::error::Error::from(mercury_connect::error::ErrorKind::Unknown)
            } )
        )
        .inspect( |_| debug!("Successfully registered to home") )
        .map_err( |e| { debug!("Failed to register: {:?}", e); std::io::Error::new( std::io::ErrorKind::ConnectionRefused, format!("{}", e) ) } );
    Box::new(init_fut)
}



pub fn init_client(client: &Client)
    -> Box< Future<Item=RelationProof, Error=std::io::Error> >
{
    let fut = init_app_common(&client.appctx)
        .map( {
            let peer_id = client.cfg.callee_profile_id.clone();
            move |my_profile| {
                let relations = my_profile.relations_with_peer( &peer_id, None,
                    Some(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN) );
                (my_profile, relations)
            }
        } )
        .and_then( {
            let peer_id = client.cfg.callee_profile_id.clone();
            let client_id = client.appctx.client_id.clone();
            let handle = client.appctx.handle.clone();
            move |(my_profile, mut relations)| {
                match relations.pop() {
                    Some(proof) => Box::new( Ok(proof).into_future() ) as Box<Future<Item=_,Error=_>>,
                    None => {
                        let rel_fut = my_profile.login()
                            .map( |session| session.events() )
                            .and_then( move |events| my_profile.initiate_relation(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN, &peer_id)
                                .map( |()| events ) )
                            .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable) )
                            .and_then( |events| wait_for_pairing_response(events, client_id, handle) );
                        Box::new(rel_fut)
                    }
                }
            }
        } );
    Box::new(fut)
}



pub fn init_server(server: &Server)
    -> Box< Future<Item=(), Error=std::io::Error> >
{
    let handle = server.appctx.handle.clone();
    let fut = init_app_common(&server.appctx)
        .and_then( move |my_profile| my_profile.login()
            .map( |session| (my_profile, session) )
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "failed to fetch events")) )
        .and_then( move |(my_profile, session)| {
            handle.spawn(
                session.events().for_each( move |event| {
                    match event {
                        ProfileEvent::PairingRequest(half_proof) => {
                            let accept_fut = my_profile.accept_relation(&half_proof)
                                .map( |_proof| () )
                                .map_err( |e| debug!("Failed to accept pairing request: {}", e) );
                            Box::new(accept_fut) as Box<Future<Item=_,Error=_>>
                        },
                        err => Box::new( Ok( debug!("Got event {:?}, ignoring it", err) ).into_future() ),
                    }
                } )
            );
            Ok( () )
        } );
    Box::new(fut)
}


pub fn wait_for_pairing_response(events: EventStream, my_profile_id: ProfileId, handle: reactor::Handle)
    -> Box< Future<Item=RelationProof, Error=std::io::Error> >
{
    let fut = events
        .filter_map( move |event|
        {
            debug!("Profile event listener got event");
            if let ProfileEvent::PairingResponse(proof) = event {
                trace!("Got pairing response, checking peer id: {:?}", proof);
                if proof.peer_id(&my_profile_id).is_ok()
                    { return Some(proof) }
            }
            return None
        } )
        .take(1)
        .into_future()
        .map_err( |((),__stream)| {
            debug!("Pairing failed");
            std::io::Error::new(std::io::ErrorKind::Other, "Pairing failed")
        } )
        .and_then( |(proof, _stream)| {
            proof.ok_or_else( || {
                debug!("Profile event stream ended without proper response");
                std::io::Error::new(std::io::ErrorKind::Other, "Got no pairing response")
            } )
        } )
        .and_then( move |proof| reactor::Timeout::new( Duration::from_millis(10), &handle ).unwrap()
            .map( |_| proof ) );
    Box::new(fut)
}
