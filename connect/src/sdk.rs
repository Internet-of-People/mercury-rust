use std::rc::Rc;

use failure::Fail;
use futures::prelude::*;
use futures::sync::mpsc;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::{Call, DAppSession, Relation, client::ProfileGateway};
use ::error::{Error, ErrorKind};



pub struct DAppConnect
{
    gateway:        Rc<ProfileGateway>,
    app_id:         ApplicationId,
}


impl DAppConnect
{
    pub fn new(gateway: Rc<ProfileGateway>, app: &ApplicationId) -> Self
        { Self{ gateway, app_id: app.to_owned() } }


    // Try fetching RelationProof from existing contacts.
    fn get_relation_proof(&self, peer_id: &ProfileId)
        -> Box< Future<Item=RelationProof, Error=Error>>
    {
        let my_id = self.gateway.signer().profile_id().to_owned();
        let peer_id = peer_id.to_owned();

        let proof_fut = self.contacts()
            .and_then( move |contacts|
            {
                let first_match = contacts.iter()
                    .map( |relation| relation.proof.to_owned() )
                    .filter( move |proof| {
                        let res = proof.peer_id(&my_id).map( |id| id.to_owned() );
                        res.is_ok() && res.unwrap() == peer_id.clone()
                    })
                    .nth(0);

                first_match.ok_or( ErrorKind::Unknown.into() )
            } );
        Box::new(proof_fut)
    }
}


// TODO this aims only feature-completeness initially for a HelloWorld dApp,
//      but we also have to include security with authorization and UI-plugins later
impl DAppSession for DAppConnect
{
    fn selected_profile(&self) -> &ProfileId
        { self.gateway.signer().profile_id() }


    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=Error> >
    {
        // TODO properly implement this
        // unimplemented!();
        Box::new( Ok( Vec::new() ).into_future() )
    }


    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=Error> >{
        unimplemented!();
    }


    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=Error> >
    {
        let checkin_fut = self.gateway.login()
            .and_then( {
                let app = self.app_id.clone();
                move |session| {
                    debug!("Checking in app {:?} to receive incoming calls", app);
                    Ok( session.checkin_app(&app) )
                }
            } )
            .map_err( |e| e.context(ErrorKind::Unknown).into() );
        Box::new(checkin_fut)
    }


    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Call, Error=Error> >
    {
        debug!("Got call request to {}", profile_id);
        let call_fut = self.get_relation_proof(profile_id)
            .inspect( |_| debug!("Got relation proof, initiate call") )
            .and_then( {
                let gateway = self.gateway.clone();
                let app_id = self.app_id.clone();
                let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);
                move |relation| gateway.call(relation.to_owned(), app_id, init_payload, Some(to_caller))
                    .map_err( |e| e.context(ErrorKind::Unknown).into() )
                    .and_then( |to_callee_opt| {
                        debug!("Got response to call");
                        match to_callee_opt {
                            None => Err( Error::from(ErrorKind::Unknown) ), // TODO
                            Some(to_callee) => Ok( Call{ sender: to_callee, receiver: from_callee } )
                        }
                    } )
            } );

        Box::new(call_fut)
    }
}
