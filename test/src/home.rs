use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::server::*;
use mercury_connect::*;
use futures::Stream;
use super::*;

#[derive(Clone)]
pub struct TestClient
{
    ownprofile: OwnProfile,
    home_context: PeerContext,
    home_connection: Rc<HomeConnectionServer>,
}

impl TestClient
{
    fn new(client_ownprofile: OwnProfile, client_signer: Rc<Signer>, home_server: Rc<HomeServer>, home_signer: Rc<Signer>, home_profile: &Profile) -> Self
    {
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
    pub reactor: reactor::Core,
    pub home_server: Rc<HomeServer>,
    pub home_signer: Rc<Signer>,
    pub home_profile: Profile,
    pub testclient: TestClient,  // not spelled as test_client, because that would be misleading, e.g. test_client_home_session
}

impl TestSetup
{
    pub fn init_direct()-> Self
    {
        let reactor = reactor::Core::new().unwrap();

        let home_server = Rc::new( default_home_server( &reactor.handle() ) );

        let (home_profile, home_signer) = generate_home();
        let home_signer = Rc::new(home_signer);

        let (testclient_ownprofile, testclient_signer) = generate_persona();

        let testclient = TestClient::new(
            testclient_ownprofile,
            Rc::new(testclient_signer),
            home_server.clone(),
            home_signer.clone(),
            &home_profile
        );

        Self { reactor, home_server, home_signer, home_profile, testclient }
    }

    pub fn init_capnp() -> Self
    {
        Self::init_direct() // TODO add capnp communication layer here
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

    let testclient2 = TestClient::new(ownprofile2, signer2.clone(), setup.home_server.clone(), setup.home_signer.clone(), &setup.home_profile.clone());
    let ownprofile2 = register_client(&mut setup, &testclient2);

    let session1 = setup.reactor.run(setup.testclient.home_connection.login(ownprofile1.profile.id.clone())).unwrap();
    let session2 = setup.reactor.run(testclient2.home_connection.login(ownprofile2.profile.id.clone())).unwrap();

    let events1 = session1.events();
    let events2 = session2.events();

    let half_proof = RelationHalfProof::new("friend", &ownprofile2.profile.id, setup.testclient.home_context.my_signer());
    let pair_result = setup.reactor.run(setup.testclient.home_connection.pair_request(half_proof)).unwrap();
    assert_eq!(pair_result, ());

    let pairing_request_event = events2.wait().next().unwrap().unwrap().unwrap();
    if let ProfileEvent::PairingRequest(half_proof) = pairing_request_event {
        assert_eq!(half_proof.peer_id, ownprofile2.profile.id);
        let proof = RelationProof::sign_remaining_half(&half_proof, &*signer2).unwrap();
        setup.reactor.run(testclient2.home_connection.pair_response(proof)).unwrap();
    } else {
        panic!();
    }

    let pairing_response_event = events1.wait().next().unwrap().unwrap().unwrap();
    if let ProfileEvent::PairingResponse(proof) = pairing_response_event {
        let validator = CompositeValidator::default();
        validator.validate_relation_proof(&proof,
            &ownprofile1.profile.id, &ownprofile1.profile.public_key,
            &ownprofile2.profile.id, &ownprofile2.profile.public_key
        ).expect("proof should be valid");
    } else {
        panic!();
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

    if let ProfileFacet::Persona(ref facet) = registered_ownprofile.profile.facets[0] {
        let home_proof = &facet.homes[0];

        assert_eq!(validator.validate_relation_proof(
            &home_proof,
            &setup.testclient.home_context.peer_id(),
            &setup.testclient.home_context.peer_pubkey(),
            &setup.testclient.ownprofile.profile.id,
            &setup.testclient.ownprofile.profile.public_key
        ), Ok(()));
    } else {
        assert!(false);
    }
}

#[test]
fn test_home_register_configs()
{
    test_home_register( TestSetup::init_direct() );
    test_home_register( TestSetup::init_capnp() );
}

#[test]
fn test_home_claim_configs()
{
    test_home_claim( TestSetup::init_direct() );
    test_home_claim( TestSetup::init_capnp() );
}

#[test]
fn test_home_events_configs()
{
    test_home_events( TestSetup::init_direct() );
    test_home_events( TestSetup::init_capnp() );
}

#[test]
fn test_home_login_configs()
{
    test_home_login( TestSetup::init_direct() );
    test_home_login( TestSetup::init_capnp() );
}

#[test]
#[ignore]
fn test_session_update()
{

}