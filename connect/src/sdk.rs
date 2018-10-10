use std::rc::Rc;

use futures::prelude::*;
use futures::sync::mpsc;

use super::*;
use profile::MyProfile;



//pub struct RelationImpl
//{
//    relation_proof: RelationProof,
//    dapp_session:   Rc<DAppSessionImpl>,
//}
//
//impl RelationImpl
//{
//    fn new(relation_proof: RelationProof, dapp_session: Rc<DAppSessionImpl>) -> Self
//        { Self { relation_proof, dapp_session } }
//}
//
//impl Relation for RelationImpl
//{
//    fn call(&self, init_payload: AppMessageFrame) -> Box< Future<Item=DAppCall, Error=Error> >
//    {
//        let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);
//
//        //self.relation_proof;
//        //let app_id = self.dapp_session.app_id.clone();
//        //let my_profile = self.dapp_session.my_profile.clone();
//        let call_fut = self.dapp_session.my_profile.call( self.relation_proof.clone(),
//                self.dapp_session.app_id.clone(), init_payload, Some(to_caller) )
//            .and_then( |to_callee_opt|
//            {
//                debug!("Call was answered, processing response");
//                match to_callee_opt {
//                    None => Err( Error::from(ErrorKind::CallRefused) ),
//                    Some(to_callee) => {
//                        info!("Call with duplex channel established");
//                        Ok( DAppCall{ outgoing: to_callee, incoming: from_callee } )
//                    }
//                }
//            } );
//
//        Box::new(call_fut)
//    }
//}



pub struct DAppSessionImpl
{
    my_profile:     Rc<MyProfile>,
    app_id:         ApplicationId,
}


impl DAppSessionImpl
{
    pub fn new(my_profile: Rc<MyProfile>, app_id: ApplicationId) -> Rc<DAppSession>
        { Rc::new( Self{ my_profile, app_id } ) }
}


// TODO this aims only feature-completeness initially for a HelloWorld dApp,
//      but we also have to include security with authorization and UI-plugins later
impl DAppSession for DAppSessionImpl
{
    fn selected_profile(&self) -> &ProfileId
        { self.my_profile.signer().profile_id() }


    fn contacts(&self) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        let mut relations = self.my_profile.relations();
        let app_contacts = relations.drain(..)
            .filter( |proof| proof.accessible_by(&self.app_id) )
//            .map( |proof| Box::new( RelationImpl::new(
//                proof, self.clone() ) ) as Box<Relation> )
            .collect();
        Box::new( Ok(app_contacts).into_future() )
    }

    fn contacts_with_profile(&self, profile: &ProfileId, relation_type: Option<&str>)
        -> Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        Box::new( Ok(
            self.my_profile.relations_with_peer(profile, Some(&self.app_id), relation_type)
//                .drain(..)
//                .map( |proof| Box::new( RelationImpl::new(
//                    proof, self.clone() ) ) as Box<Relation> )
//                .collect()
        ).into_future() )
    }

    fn initiate_contact(&self, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >
    {
        // TODO relation-type should be more sophisticated once we have a proper metainfo schema there
        self.my_profile.initiate_relation(&self.app_id.0, with_profile)
    }


    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=Error> >
        { unimplemented!(); }


    fn checkin(&self)
        -> Box< Future<Item=Box<Stream<Item=DAppEvent, Error=()>>, Error=Error> >
    {
        let app = self.app_id.clone();
        let fut = self.my_profile.login()
            .map( move |my_session|
            {
                let app1 = app.clone();
                let calls_stream = my_session.session().checkin_app(&app)
                    .inspect( move |_| debug!("Checked in app {:?} to receive incoming calls", app1) )
                    // Filter stream elements, keep only successful calls, drop errors
                    .filter_map( |inc_call_res| inc_call_res.ok() )
                    // Transform only Ok(call) into an event
                    .map( |call| DAppEvent::Call(call) );

                let app2 = app.clone();
                let events_stream = my_session.events()
                    .inspect( move |_| debug!("Forwarding events related to app {:?}", app2) )
                    .filter_map( move |event|
                        match event {
                            ProfileEvent::PairingResponse(ref proof) if proof.accessible_by(&app) =>
                                Some( DAppEvent::PairingResponse( proof.clone() ) ),
                            _ => None
                        }
                    );

                Box::new( calls_stream.select(events_stream) ) as Box<Stream<Item=_,Error=_>>
            } );

        Box::new(fut)
    }


    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=DAppCall, Error=Error> >
    {
        debug!("Looking for relation proof to call profile {}", profile_id);
        let relation_res = self.my_profile.relations_with_peer(profile_id, Some(&self.app_id), None )
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
