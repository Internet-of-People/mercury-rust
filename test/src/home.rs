use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::server::*;
use mercury_connect::*;
use super::*;


pub struct TestSetup
{
    pub reactor: reactor::Core,
    pub client_ownprofile: OwnProfile,
    pub client_home_context: PeerContext,
    pub home: Rc<Home>,
}

impl TestSetup
{
    pub fn init_direct()-> Self
    {
        let reactor = reactor::Core::new().unwrap();

        let server = Rc::new( default_home_server( &reactor.handle() ) );

        let home_facet = ProfileFacet::Home( HomeFacet{ addrs: vec![] , data: Vec::new() } );
        let (home_ownprofile, home_signer) = generate_ownprofile(home_facet, vec![]);

        let persona_facet = ProfileFacet::Persona( PersonaFacet{ homes: vec![] , data: Vec::new() } );
        let (client_ownprofile, client_signer) = generate_ownprofile(persona_facet, vec![]);

        let server_context = Rc::new(PeerContext::new( Rc::new(home_signer),
            client_ownprofile.profile.public_key.clone(), client_ownprofile.profile.id.clone() ) );
        let home = HomeConnectionServer::new(server_context, server).unwrap();

        let client_home_context = PeerContext::new_from_profile( Rc::new(client_signer), &home_ownprofile.profile);

        Self{ reactor, client_ownprofile, client_home_context, home: Rc::new(home)}
    }

    pub fn init_capnp() -> Self
    {
        Self::init_direct() // TODO add capnp communication layer here
    }
}

fn register_client(setup: &mut TestSetup) -> OwnProfile
{
    let half_proof = RelationHalfProof::new( "home", setup.client_home_context.peer_id(), setup.client_home_context.my_signer() );
    let reg_fut = setup.home.register(setup.client_ownprofile.clone(), half_proof, None);
    setup.reactor.run(reg_fut).unwrap()
}

fn test_home_claim(mut setup: TestSetup)
{
    let registered_ownprofile = register_client(&mut setup);

    let claim_fut = setup.home.claim(setup.client_ownprofile.profile.id.clone());
    let claimed_ownprofile = setup.reactor.run(claim_fut).unwrap();

    assert_eq!(registered_ownprofile, claimed_ownprofile);
}

fn test_home_register(mut setup: TestSetup)
{
    let registered_ownprofile = register_client(&mut setup);
    let validator = CompositeValidator::default();

    if let ProfileFacet::Persona(ref facet) = registered_ownprofile.profile.facets[0] {
        let home_proof = &facet.homes[0];

        assert_eq!(validator.validate_relation_proof(
            &home_proof,
            &setup.client_home_context.peer_id(),
            &setup.client_home_context.peer_pubkey(),
            &setup.client_ownprofile.profile.id,
            &setup.client_ownprofile.profile.public_key
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
fn test_home_login()
{

}

#[test]
fn test_session_update()
{

}