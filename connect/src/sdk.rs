use std::rc::Rc;

use futures::prelude::*;
use futures::sync::mpsc;

use super::*;
use profile::MyProfile;



pub struct DAppConnect
{
    my_profile:     Rc<MyProfile>,
    app_id:         ApplicationId,
}


impl DAppConnect
{
    pub fn new(my_profile: Rc<MyProfile>, app_id: ApplicationId) -> Rc<DAppSession>
        { Rc::new( Self{ my_profile, app_id } ) }
}


// TODO this aims only feature-completeness initially for a HelloWorld dApp,
//      but we also have to include security with authorization and UI-plugins later
impl DAppSession for DAppConnect
{
    fn selected_profile(&self) -> &ProfileId
        { self.my_profile.signer().profile_id() }


    fn contacts(&self) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        // TODO properly implement this to access only contacts related to this dApp
        Box::new(Ok( self.my_profile.relations() ).into_future() )
        // unimplemented!();
    }


    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=Error> >{
        unimplemented!();
    }


    fn checkin(&self)
        -> Box< Future<Item=Box<Stream<Item=Result<DAppEvent,String>, Error=()>>, Error=::Error> >
    {
        let checkin_fut = self.my_profile.login()
            .and_then(
            {
                let app = self.app_id.clone();
                move |my_session| {
                    debug!("Checking in app {:?} to receive incoming calls", app);
                    let event_stream = my_session.session().checkin_app(&app)
                        // Map stream elements, i.e. each incoming call Result object
                        .map( |inc_call_res| inc_call_res
                            // Transform only Ok(call) into an event
                            .map( |call| DAppEvent::Call(call) ) );
                    Ok( Box::new(event_stream) as Box<Stream<Item=_,Error=_>>)
                }
            } );
        Box::new(checkin_fut)
    }


    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=DAppCall, Error=Error> >
    {
        debug!("Looking for relation proof to call profile {}", profile_id);
        let relation_res = self.my_profile.relations_with_peer(profile_id,
                Some(&self.app_id), Some(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN) )
            .pop()
            .ok_or_else( || {
                debug!("Failed to find proper relationproof to start call, drop call request");
                ErrorKind::FailedToAuthorize.into()
            } );

        let call_fut = relation_res.into_future()
            .and_then(
            {
                let my_profile = self.my_profile.clone();
                let app_id = self.app_id.clone();
                let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);
                move |relation| my_profile.call(relation.to_owned(), app_id, init_payload, Some(to_caller))
                    .and_then( |to_callee_opt| {
                        debug!("Call was answered, processing response");
                        match to_callee_opt {
                            None => Err( Error::from(ErrorKind::CallRefused) ),
                            Some(to_callee) => {
                                info!("Call with duplex channel established");
                                Ok( DAppCall{ outgoing: to_callee, incoming: from_callee } )
                            }
                        }
                    } )
            } );

        Box::new(call_fut)
    }
}
