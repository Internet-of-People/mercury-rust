use super::*;

use std::collections::HashSet;

use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::simple_profile_repo::SimpleProfileRepo;
use mercury_connect::sdk::{DAppApi, Call};
use mercury_connect::{Relation, ProfileGatewayImpl, ProfileGateway};
use mercury_home_protocol::{ProfileId, AppMessageFrame, HomeStream, IncomingCall, ErrorToBeSpecified};
use mercury_home_protocol::crypto::Ed25519Signer;
use mercury_storage::{async::KeyValueStore, filesys::AsyncFileHandler};

//TODO 0.2
//1. make profile
//2. register profile on mercury home node
//3. pair server and client
//4. make call from client towards server to declare "active state"
//5. send event(s) from server to client(s) in active state

struct DappConnect{
    contacts: Vec<Relation>,
    gateway: ProfileGatewayImpl, 
    storage: AsyncFileHandler,
    homesession : HomeSession //no client side implementation
}

impl DAppApi for DappConnect{
    // Implies asking the user interface to manually pick a profile the app is used with
    fn new(proposed_profile_id: Option<ProfileId>)
        -> Box< Future<Item=Box<Self>, Error=ErrorToBeSpecified> >{
        
        let pg = ProfileGatewayImpl::new(
            Rc::new(Ed25519Signer::new(/*needs priv key*/)),
            Rc::new(SimpleProfileRepo::new()),
            Rc::new(SimpleTcpHomeConnector::new(/*needs handle*/))
        );
        pg.login().and_then(|homesession|{
            Self{
                contacts: Vec::new(),
                gateway: pg, 
                storage: AsyncFileHandler::new(),
                homesession : HomeSession //no client side implementation
            }
        })
    }

    // Once initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> &ProfileId{
        unimplemented!();
    }

    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >{
        unimplemented!();
    }

    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=ErrorToBeSpecified> >{
        unimplemented!();
    }

    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=ErrorToBeSpecified> >{
        unimplemented!();
    }

    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Box<Call>, Error=ErrorToBeSpecified> >{
        unimplemented!();
    }
}
