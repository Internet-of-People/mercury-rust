use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::server::*;
use super::*;



pub struct TestSetup
{
    pub reactor: reactor::Core,
    pub client_ownprofile: OwnProfile,
    pub home: Rc<RefCell<Home>>,
}

impl TestSetup
{
    pub fn init_direct()-> Self
    {
        let reactor = reactor::Core::new().unwrap();

        let server = Rc::new( default_home_server( &reactor.handle() ) );

        let home_facet = ProfileFacet::Home( HomeFacet{ addrs: vec![] , data: Vec::new() } );
        let (_home_ownprofile, home_signer) = generate_ownprofile(home_facet, vec![]);

        let persona_facet = ProfileFacet::Persona( PersonaFacet{ homes: vec![] , data: Vec::new() } );
        let (client_ownprofile, _client_signer) = generate_ownprofile(persona_facet, vec![]);

        let context = Rc::new(ClientContext::new( Rc::new(home_signer),
            client_ownprofile.profile.pub_key.clone(), client_ownprofile.profile.id.clone() ) );
        let home = HomeConnectionServer::new(context, server).unwrap();

        Self{ reactor, client_ownprofile, home: Rc::new( RefCell::new(home) ) }
    }
}



#[test]
fn test_registration()
{
    let setup = TestSetup::init_direct();
//    let halfproof = RelationHalfProof::WHAT?();
//    let reg_fut = setup.home.register(setup.client_ownprofile, );
}
