use super::*;

use std::rc::Rc;

use futures::sync::mpsc;

use sdk::*;
use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;



pub struct DAppConnect
{
    pub gateway: Rc<ProfileGateway>,
    pub app:     ApplicationId
}


impl DAppApi for DAppConnect
{
    fn selected_profile(&self) -> &ProfileId
        { self.gateway.signer().profile_id() }


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
        let my_id = self.gateway.signer().profile_id().to_owned();
        let profile_id = profile_id.to_owned();
        let call_fut = self.contacts()
            .and_then( |contacts|
            {
                let first_match = contacts.iter()
                    .filter( move |relation| relation.proof.peer_id(&my_id).map(|id| id.to_owned()) == Ok(profile_id.clone()) )
                    .nth(0);
                match first_match {
                    // TODO If no appropriate contact found, we should initiate a pairing request instead
                    None => Err( ErrorToBeSpecified::TODO( "call(): no matching contact was found".to_string() ) ),
                    Some(relation) => Ok(relation.to_owned())
                }
            } )
            .and_then(
            {
                let gateway = self.gateway.clone();
                let app_id = self.app.clone();
                let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);
                move |relation| gateway.call(relation.to_owned(), app_id, init_payload, Some(to_caller))
                    .and_then( |to_callee_opt|
                        match to_callee_opt {
                            None => Err( ErrorToBeSpecified::TODO( "call was refused be the callee".to_string() ) ),
                            Some(to_callee) => Ok( Call{ sender: to_callee, receiver: from_callee } )
                        }
                    )
            } );
        
        Box::new(call_fut)
    }
}
