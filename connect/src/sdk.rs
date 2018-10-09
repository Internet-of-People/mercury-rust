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
        let mut relations = self.my_profile.relations();
        let app_contacts = relations.drain(..)
            .filter( |proof| proof.accessible_by(&self.app_id) )
            .collect();
        Box::new( Ok(app_contacts).into_future() )
    }

    fn contacts_with_profile(&self, profile: &ProfileId, relation_type: Option<&str>)
        -> Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        Box::new( Ok(
            self.my_profile.relations_with_peer(profile, Some(&self.app_id), relation_type)
        ).into_future() )
    }


    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=Error> >
        { unimplemented!(); }


    fn checkin(&self)
        -> Box< Future<Item=Box<Stream<Item=Result<DAppEvent,String>, Error=()>>, Error=::Error> >
    {
        let app = self.app_id.clone();
        let calls_fut = self.my_profile.login()
            .map( move |my_session|
            {
                debug!("Checking in app {:?} to receive incoming calls", app);
                my_session.session().checkin_app(&app)
                    // Map stream elements, i.e. each incoming call Result object
                    .map( |inc_call_res| inc_call_res
                        // Transform only Ok(call) into an event
                        .map( |call| DAppEvent::Call(call) ) )
            } );

        let app = self.app_id.clone();
        let pair_resps_fut = self.my_profile.login()
            .map( |my_session|
            {
                debug!("Forwarding events related to app {:?}", app);
                my_session.events()
                    .filter_map( move |event|
                        match event {
                            ProfileEvent::PairingResponse(ref proof) if proof.accessible_by(&app) =>
                                Some( Ok( DAppEvent::PairingResponse( proof.clone() ) ) ),
                            _ => None
                        }
                    )
            } );

        // Merge the two streams into one
        let both_fut = calls_fut.join(pair_resps_fut)
            .map( |(calls_stream, proofs_stream)|
                Box::new( calls_stream.select(proofs_stream) ) as Box<Stream<Item=_,Error=_>> );

        Box::new(both_fut)
    }


    fn initiate_relation(&self, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >
    {
        // TODO relation-type should be more sophisticated once we have a proper metainfo schema there
        self.my_profile.initiate_relation(&self.app_id.0, with_profile)
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
