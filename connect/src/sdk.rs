use std::rc::Rc;

use futures::prelude::*;
use futures::sync::mpsc;
use log::*;

use super::*;
use profile::MyProfile;

pub struct RelationImpl {
    relation_proof: RelationProof,
    my_profile: Rc<dyn MyProfile>,
    app_id: ApplicationId,
}

impl RelationImpl {
    fn new(
        relation_proof: RelationProof,
        my_profile: Rc<dyn MyProfile>,
        app_id: ApplicationId,
    ) -> Self {
        Self { relation_proof, my_profile, app_id }
    }
}

impl Contact for RelationImpl {
    fn proof(&self) -> &RelationProof {
        &self.relation_proof
    }

    fn call(&self, init_payload: AppMessageFrame) -> AsyncResult<DAppCall, Error> {
        let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);

        let call_fut = self
            .my_profile
            .call(self.relation_proof.clone(), self.app_id.clone(), init_payload, Some(to_caller))
            .and_then(|to_callee_opt| {
                debug!("Call was answered, processing response");
                match to_callee_opt {
                    None => Err(Error::from(ErrorKind::CallRefused)),
                    Some(to_callee) => {
                        info!("Call with duplex channel established");
                        Ok(DAppCall { outgoing: to_callee, incoming: from_callee })
                    }
                }
            });

        Box::new(call_fut)
    }
}

pub struct DAppSessionImpl {
    my_profile: Rc<dyn MyProfile>,
    app_id: ApplicationId,
}

impl DAppSessionImpl {
    pub fn new(my_profile: Rc<dyn MyProfile>, app_id: ApplicationId) -> Rc<dyn DAppSession> {
        Rc::new(Self { my_profile, app_id })
    }

    fn relation_from(&self, proof: RelationProof) -> Box<dyn Contact> {
        Self::relation_from2(proof, self.my_profile.clone(), self.app_id.clone())
    }

    fn relation_from2(
        proof: RelationProof,
        my_profile: Rc<dyn MyProfile>,
        app_id: ApplicationId,
    ) -> Box<dyn Contact> {
        Box::new(RelationImpl::new(proof, my_profile, app_id))
    }
}

// TODO this aims only feature-completeness initially for a HelloWorld dApp,
//      but we also have to include security with authorization and UI-plugins later
impl DAppSession for DAppSessionImpl {
    fn selected_profile(&self) -> ProfileId {
        self.my_profile.signer().profile_id()
    }

    fn contacts(&self) -> AsyncResult<Vec<Box<dyn Contact>>, Error> {
        let mut proofs = self.my_profile.relations();
        let app_contacts = proofs
            .drain(..)
            .filter(|proof| proof.accessible_by(&self.app_id))
            .map(|proof| self.relation_from(proof))
            .collect();
        Box::new(Ok(app_contacts).into_future())
    }

    fn contacts_with_profile(
        &self,
        profile: &ProfileId,
        relation_type: Option<&str>,
    ) -> AsyncResult<Vec<Box<dyn Contact>>, Error> {
        let mut proofs =
            self.my_profile.relations_with_peer(profile, Some(&self.app_id), relation_type);
        let peer_contacts = proofs.drain(..).map(|proof| self.relation_from(proof)).collect();
        Box::new(Ok(peer_contacts).into_future())
    }

    fn initiate_contact(&self, with_profile: &ProfileId) -> AsyncResult<(), Error> {
        // TODO relation-type should be more sophisticated once we have a proper metainfo schema there
        self.my_profile.initiate_relation(&self.app_id.0, with_profile)
    }

    fn app_storage(&self) -> AsyncResult<dyn KeyValueStore<String, String>, Error> {
        unimplemented!();
    }

    fn checkin(&self) -> AsyncResult<Box<dyn Stream<Item = DAppEvent, Error = ()>>, Error> {
        let app = self.app_id.clone();
        let my_profile = self.my_profile.clone();
        let fut = self.my_profile.login().map(move |my_session| {
            let app1 = app.clone();
            let calls_stream = my_session
                .session()
                .checkin_app(&app)
                .inspect(move |_| debug!("Checked in app {:?} to receive incoming calls", app1))
                // Filter stream elements, keep only successful calls, drop errors
                .filter_map(|inc_call_res| inc_call_res.ok())
                // Transform only Ok(call) into an event
                .map(|call| DAppEvent::Call(call));

            let app2 = app.clone();
            let app3 = app.clone();
            let events_stream = my_session
                .events()
                .inspect(move |_| debug!("Forwarding events related to app {:?}", app2))
                .filter_map(move |event| match event {
                    ProfileEvent::PairingResponse(ref proof) if proof.accessible_by(&app) => {
                        Some(DAppEvent::PairingResponse(Self::relation_from2(
                            proof.clone(),
                            my_profile.clone(),
                            app3.clone(),
                        )))
                    }
                    _ => None,
                });

            Box::new(calls_stream.select(events_stream)) as Box<dyn Stream<Item = _, Error = _>>
        });

        Box::new(fut)
    }
}
