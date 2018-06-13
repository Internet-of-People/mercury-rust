use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::{ server::*, protocol_capnp::HomeDispatcherCapnProto };
use mercury_connect::{ *, protocol_capnp::HomeClientCapnProto };

use futures::Stream;
use super::*;

#[derive(Clone)]
pub struct TestClient
{
    ownprofile: OwnProfile,
    home_context: PeerContext,
    home_connection: Rc<Home>,
}

#[derive(Clone)]
pub enum TestMode {
    Direct,
    Memsocket,
}

impl TestClient
{

    fn new(
        test_mode: TestMode,
        ownprofile: OwnProfile,
        client_signer: Rc<Signer>,
        home_server: Rc<HomeServer>,
        home_signer: Rc<Signer>,
        home_profile: &Profile,
        handle: reactor::Handle
    ) -> Self {
        match test_mode {
            TestMode::Direct => Self::direct(ownprofile, client_signer, home_server, home_signer, home_profile),
            TestMode::Memsocket => Self::memsocket(ownprofile, client_signer, home_server, home_signer, home_profile, handle),
        }
    }

    fn memsocket(
        ownprofile: OwnProfile,
        client_signer: Rc<Signer>,
        home_server: Rc<HomeServer>,
        home_signer: Rc<Signer>,
        home_profile: &Profile,
        handle: reactor::Handle
    ) -> Self {

        let (receiver_from_client, sender_from_client) = memsocket::unbounded();  // client to server
        let (receiver_from_server, sender_from_server) = memsocket::unbounded();  // server to client

        // server
        let home_client_context = Rc::new(PeerContext::new(
            home_signer.clone(),
            ownprofile.profile.public_key.clone(),
            ownprofile.profile.id.clone()
        ));

        let home_connection = Rc::new(HomeConnectionServer::new(home_client_context, home_server.clone()).unwrap());

        HomeDispatcherCapnProto::dispatch(
            home_connection,
            receiver_from_client,
            sender_from_server,
            handle.clone()
        );

        // client
        let home_context = PeerContext::new_from_profile(client_signer.clone(), &home_profile);

        let client_capnp = HomeClientCapnProto::new(
            receiver_from_server,
            sender_from_client,
            home_context.clone(),
            handle.clone()
        );

        TestClient {
            ownprofile: ownprofile.clone(),
            home_context,
            home_connection: Rc::new(client_capnp),
        }
    }

    fn direct(
        client_ownprofile: OwnProfile,
        client_signer: Rc<Signer>,
        home_server: Rc<HomeServer>,
        home_signer: Rc<Signer>,
        home_profile: &Profile
    ) -> Self {
        let home_client_context = Rc::new(PeerContext::new(
            home_signer.clone(),
            client_ownprofile.profile.public_key.clone(),
            client_ownprofile.profile.id.clone()
        ));

        let client_home_connection = Rc::new(HomeConnectionServer::new(home_client_context, home_server.clone()).unwrap());
        let client_home_context = PeerContext::new_from_profile(client_signer.clone(), &home_profile);

        TestClient {
            ownprofile: client_ownprofile,
            home_context: client_home_context,
            home_connection: client_home_connection,
        }
    }
}


pub struct TestSetup
{
    pub mode: TestMode,
    pub reactor: reactor::Core,
    pub home_server: Rc<HomeServer>,
    pub home_signer: Rc<Signer>,
    pub home_profile: Profile,
    pub testclient: TestClient,  // not spelled as test_client, because that would be misleading, e.g. test_client_home_session
}

impl TestSetup
{
    pub fn init(mode: TestMode)-> Self
    {
        let reactor = reactor::Core::new().unwrap();

        let home_server = Rc::new( default_home_server( &reactor.handle() ) );

        let (home_profile, home_signer) = generate_home();
        let home_signer = Rc::new(home_signer);

        let (testclient_ownprofile, testclient_signer) = generate_persona();

        let testclient = TestClient::new(
            mode.clone(),
            testclient_ownprofile,
            Rc::new(testclient_signer),
            home_server.clone(),
            home_signer.clone(),
            &home_profile,
            reactor.handle()
        );
        Self { mode, reactor, home_server, home_signer, home_profile, testclient }
    }
}

fn register_client(setup: &mut TestSetup, client: &TestClient) -> OwnProfile
{
    let half_proof = RelationHalfProof::new("home", setup.testclient.home_context.peer_id(), client.home_context.my_signer());
    let reg_fut = client.home_connection.register(client.ownprofile.clone(), half_proof, None);
    setup.reactor.run(reg_fut).unwrap()
}

fn register_client_from_setup(setup: &mut TestSetup) -> OwnProfile
{
    let testclient = setup.testclient.clone();
    register_client(setup, &testclient)
}

fn test_home_events(mut setup: TestSetup)
{
    let ownprofile1 = register_client_from_setup(&mut setup);

    let (ownprofile2, signer2) = generate_persona();
    let signer2 = Rc::new(signer2);

    let home_server_clone = setup.home_server.clone();
    let home_signer_clone = setup.home_signer.clone();
    let home_profile_clone = setup.home_profile.clone();
    let testclient2 = TestClient::new(
        setup.mode.clone(),
        ownprofile2, signer2.clone(),
        home_server_clone, home_signer_clone, &home_profile_clone,
        setup.reactor.handle()
    );
    let ownprofile2 = register_client(&mut setup, &testclient2);

    let session1 = setup.reactor.run(setup.testclient.home_connection.login(ownprofile1.profile.id.clone())).unwrap();
    let session2 = setup.reactor.run(testclient2.home_connection.login(ownprofile2.profile.id.clone())).unwrap();

    let events1 = session1.events();
    let events2 = session2.events();

    let half_proof = RelationHalfProof::new("friend", &ownprofile2.profile.id, setup.testclient.home_context.my_signer());
    let pair_result = setup.reactor.run(setup.testclient.home_connection.pair_request(half_proof)).unwrap();
    assert_eq!(pair_result, ());

    let events_fut = events2.take(1).collect();
    let single_event: Vec<Result<ProfileEvent, String>> = setup.reactor.run(events_fut).unwrap();
    let pairing_request_event = single_event.get(0).unwrap().clone().unwrap();

    match pairing_request_event {
        ProfileEvent::PairingRequest(half_proof) => {
            assert_eq!(half_proof.peer_id, ownprofile2.profile.id);

            let proof = RelationProof::sign_remaining_half(&half_proof, &*signer2).unwrap();
            setup.reactor.run(testclient2.home_connection.pair_response(proof)).unwrap();
        },
        _ => panic!("not a PairingRequest"),
    }

    let events_fut = events1.take(1).collect();
    let single_event = setup.reactor.run(events_fut).unwrap();
    let pairing_response_event = single_event.get(0).unwrap().clone().unwrap();

    match pairing_response_event {
        ProfileEvent::PairingResponse(proof) => {
            let validator = CompositeValidator::default();
            validator.validate_relation_proof(&proof,
                &ownprofile1.profile.id, &ownprofile1.profile.public_key,
                &ownprofile2.profile.id, &ownprofile2.profile.public_key
            ).expect("proof should be valid");
        },
        _ => panic!("not a PairingResponse"),
    }
}

fn test_home_login(mut setup: TestSetup)
{
    let ownprofile = register_client_from_setup(&mut setup);
    let session = setup.reactor.run(setup.testclient.home_connection.login(ownprofile.profile.id)).unwrap();
    let pong = setup.reactor.run(session.ping("ping")).unwrap();
    assert_eq!("ping", pong);
}

fn test_home_claim(mut setup: TestSetup)
{
    let registered_ownprofile = register_client_from_setup(&mut setup);

    let claim_fut = setup.testclient.home_connection.claim(setup.testclient.ownprofile.profile.id.clone());
    let claimed_ownprofile = setup.reactor.run(claim_fut).unwrap();

    assert_eq!(registered_ownprofile, claimed_ownprofile);
}

fn test_home_register(mut setup: TestSetup)
{
    let registered_ownprofile = register_client_from_setup(&mut setup);
    let validator = CompositeValidator::default();

    match registered_ownprofile.profile.facet {
        ProfileFacet::Persona(ref facet) => {
            let home_proof = &facet.homes[0];

            assert_eq!(validator.validate_relation_proof(
                &home_proof,
                &setup.testclient.home_context.peer_id(),
                &setup.testclient.home_context.peer_pubkey(),
                &setup.testclient.ownprofile.profile.id,
                &setup.testclient.ownprofile.profile.public_key
            ), Ok(()));
        },
        _ => panic!(),
    }
}

#[test]
fn test_home_register_configs()
{
    test_home_register( TestSetup::init(TestMode::Direct) );
    test_home_register( TestSetup::init(TestMode::Memsocket) );
}

#[test]
fn test_home_claim_configs()
{
    test_home_claim( TestSetup::init(TestMode::Direct) );
    test_home_claim( TestSetup::init(TestMode::Memsocket) );
}

#[test]
fn test_home_events_configs()
{
    test_home_events( TestSetup::init(TestMode::Direct) );
    test_home_events( TestSetup::init(TestMode::Memsocket) );
}

#[test]
fn test_home_login_configs()
{
    test_home_login( TestSetup::init(TestMode::Direct) );
    test_home_login( TestSetup::init(TestMode::Memsocket) );
}

#[test]
#[ignore]
fn test_session_update()
{

}