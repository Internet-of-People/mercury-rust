use super::*;

use std::collections::HashSet;
use std::rc::Rc;

use mercury_connect::net::SimpleTcpHomeConnector;
use mercury_connect::simple_profile_repo::SimpleProfileRepo;
use mercury_connect::sdk::{DAppApi, Call};
use mercury_connect::{Relation, ProfileGatewayImpl, ProfileGateway};
use mercury_home_protocol::*;
use mercury_home_protocol::crypto::Ed25519Signer;
use mercury_storage::{async::KeyValueStore, filesys::AsyncFileHandler};

//TODO 0.2
//1. make profile
//2. register profile on mercury home node
//3. pair server and client
//4. make call from client towards server to declare "active state"
//5. send event(s) from server to client(s) in active state

pub struct DappConnect{
    contacts: Vec<Relation>,
    gateway: ProfileGatewayImpl, 
    storage: AsyncFileHandler,
    homesession : Box<HomeSession> //no client side implementation
}

impl DappConnect {

    /// Need to provide:
    ///  - ProfileId
    ///  - ProfileRepo
    ///  - HomeConnector
    ///  - 

    fn new(private_key: PrivateKey, profile_repo : Box<ProfileRepo> )
        -> Self {

        /*
        let pg = ProfileGatewayImpl::new(
            Rc::new(Ed25519Signer::new(private_key)),
            Rc::new(),
            Rc::new(SimpleTcpHomeConnector::new(/*needs handle*/))
        );

        */
        unimplemented!();
    }
}

impl DAppApi for DappConnect{

    // Implies asking the user interface to manually pick a profile the app is used with

    fn connect(&self, profile : Option<ProfileId>)
        -> Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        /*
        pg.login().and_then(|homesession|{
            Self{
                contacts: Vec::new(),
                gateway: pg, 
                storage: AsyncFileHandler::new(),
                homesession : Box::new(homesession) //no client side implementation
            }
        })

        */    
        unimplemented!();
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
