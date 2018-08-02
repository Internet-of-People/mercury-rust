use super::*;

use std::rc::Rc;

use sdk::*;
use mercury_storage::async::KeyValueStore;



pub struct DAppConnect
{
    pub gateway: Rc<ProfileGateway>,
    pub app:     ApplicationId
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


    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=ErrorToBeSpecified> >
    {
        let checkin_fut = self.gateway.login()
            .and_then( {
                let app = self.app.clone();
                move |session| Ok( session.checkin_app(&app) )
            } );
        Box::new(checkin_fut)
    }


    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Call, Error=ErrorToBeSpecified> >
    {
        unimplemented!();
    }
}
