use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_node::server::*;
use super::*;



pub struct TestSetup
{
    pub reactor: reactor::Core,
//    pub home: Rc<RefCell<Home>>,
//    pub userownprofile: OwnProfile,
}

impl TestSetup
{
    pub fn setup()-> Self
    {
        let reactor = reactor::Core::new().unwrap();
        let server = Rc::new( default_home( &reactor.handle() ) );
//        let (client_profile, client_signer) = generate_profile();
//        let context = Rc::new(ClientContext::new( signer.clone(), client_pub_key, client_profile_id ) );
//        let home = HomeConnectionServer::new(context, server);

        Self{ reactor,
//              home: Rc::new( RefCell::new(server) ),
//              userownprofile: userownprofile,
        }
    }
}
