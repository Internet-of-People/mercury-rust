use super::*;

use std::rc::Rc;

use sdk::*;
use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;

//TODO 0.2
//1. make profile
//2. register profile on mercury home node
//3. pair server and client
//4. make call from client towards server to declare "active state"
//5. send event(s) from server to client(s) in active state

struct DAppConnect
{
    gateway: Rc<ProfileGateway>,
}


impl DAppConnect
{
    fn new(profile_repo : Rc<ProfileGateway>) -> Self
    {
        unimplemented!();
    }
}


impl DAppInit for DAppConnect
{
    fn initialize(&self, app: &ApplicationId)
        -> Box< Future<Item=Rc<DAppApi>, Error=ErrorToBeSpecified> >
    {
        unimplemented!();
    }
}


impl DAppApi for DAppConnect
{
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
        -> Box< Future<Item=Box<Call>, Error=ErrorToBeSpecified> >
    {
        unimplemented!();
    }
}
