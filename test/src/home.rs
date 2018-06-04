use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::server::*;
use mercury_connect::*;
use super::*;



pub struct TestSetup
{
    pub reactor: reactor::Core,
    pub client_ownprofile: OwnProfile,
    pub home_context: HomeContext,
    pub home: Rc<RefCell<Home>>,
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

        let server_context = Rc::new(ClientContext::new( Rc::new(home_signer),
            client_ownprofile.profile.public_key.clone(), client_ownprofile.profile.id.clone() ) );
        let home = HomeConnectionServer::new(server_context, server).unwrap();

        let client_context = HomeContext::new( Rc::new(client_signer), &home_ownprofile.profile);

        Self{ reactor, client_ownprofile, home_context: client_context,
              home: Rc::new( RefCell::new(home) ) }
    }

    pub fn init_capnp() -> Self
    {
        Self::init_direct() // TODO add capnp communication layer here
    }
}



fn test_home_register(mut setup: TestSetup)
{
    let halfproof = RelationHalfProof::new( "home", setup.home_context.peer_id(), setup.home_context.my_signer() );
    let reg_fut = setup.home.borrow().register(setup.client_ownprofile, halfproof, None);
    let reg_res = setup.reactor.run(reg_fut).unwrap();
    // TODO assert that resulted profile contains new home in persona facet
}

#[test]
fn test_home_register_configs()
{
    test_home_register( TestSetup::init_direct() );
    test_home_register( TestSetup::init_capnp() );
}


#[test]
fn test_home_login()
{

}


#[test]
fn test_session_update()
{

}