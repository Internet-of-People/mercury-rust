use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

use failure::Fail;
use futures::prelude::*;
use futures::{future, sync::mpsc};
use log::*;
use tokio_core::reactor;

use crate::*;
use mercury_home_protocol::future as fut;
use mercury_home_protocol::net::HomeConnector;

pub trait MyProfile {
    //fn own_profile(&self) -> &OwnProfile;
    fn signer(&self) -> &Signer;

    fn homes(&self) -> AsyncResult<Vec<RelationProof>, Error>;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    /// `invite` is needed only if the home has a restrictive registration policy.
    fn join_home(
        &self,
        home: ProfileId, //invite: Option<HomeInvitation>
    ) -> AsyncResult<(), Error>;
    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
    fn leave_home(&self, home: ProfileId, newhome: Option<Profile>) -> AsyncResult<(), Error>;
    //    fn home_endpoint_hint(&self, home: &ProfileId, endpoint: multiaddr);
    //    fn profile_home_hint(&self, profile: &ProfileId, home: &ProfileId);

    fn relations(&self) -> Vec<RelationProof>;
    fn relations_with_peer(
        &self,
        peer_id: &ProfileId,
        app_filter: Option<&ApplicationId>,
        relation_type_filter: Option<&str>,
    ) -> Vec<RelationProof>;
    fn initiate_relation(
        &self,
        relation_type: &str,
        with_profile_id: &ProfileId,
    ) -> AsyncResult<(), Error>;
    fn accept_relation(&self, half_proof: &RelationHalfProof) -> AsyncResult<RelationProof, Error>;
    //    fn revoke_relation(&self, relation: &RelationProof) -> AsyncResult<(), Error>;

    fn call(
        &self,
        rel: RelationProof,
        app: ApplicationId,
        init_payload: AppMessageFrame,
        to_caller: Option<AppMsgSink>,
    ) -> AsyncResult<Option<AppMsgSink>, Error>;

    fn login(&self) -> AsyncResult<Rc<MyHomeSession>, Error>;

    //    fn events(&self, profile: &ProfileId) -> AsyncResult<EventStream, Error>;
    //    fn update(&self, home: ProfileId, own_prof: &OwnProfile) -> AsyncResult<(), Error>;
}

pub type EventSink = mpsc::Sender<ProfileEvent>;
pub type EventStream = mpsc::Receiver<ProfileEvent>;

// TODO consider if event listeners should be handled here or we should delete this and
//      allow event listeners somewhere under the service instead
pub trait MyHomeSession {
    fn session(&self) -> Rc<HomeSession>;
    fn events(&self) -> EventStream;
}

#[derive(Clone)]
pub struct MyProfileImpl {
    own_profile: Rc<RefCell<OwnProfile>>,
    signer: Rc<Signer>,
    profile_repo: Rc<ProfileExplorer>,
    home_connector: Rc<HomeConnector>,
    handle: reactor::Handle,
    session_cache: Rc<RefCell<HashMap<ProfileId, Rc<MyHomeSession>>>>, // {home_id -> session}
    // on_updated:     Rc< Fn(&OwnProfile) -> AsyncResult<(),Error>,
    // TODO remove this after testing, this should be fetched from the private binary part of OwnProfile
    relations: Rc<RefCell<Vec<RelationProof>>>,
}

impl MyProfileImpl {
    pub fn new(
        own_profile: OwnProfile,
        signer: Rc<Signer>,
        profile_repo: Rc<ProfileExplorer>,
        home_connector: Rc<HomeConnector>,
        handle: reactor::Handle,
    ) -> Self {
        Self {
            own_profile: Rc::new(RefCell::new(own_profile)),
            signer,
            profile_repo,
            home_connector,
            handle,
            relations: Default::default(),
            session_cache: Default::default(),
        }
    }

    //    pub fn new<F>(own_profile: OwnProfile, signer: Rc<Signer>, profile_repo: Rc<ProfileRepo>,
    //                  home_connector: Rc<HomeConnector>, handle: reactor::Handle,
    //                  on_updated: F) -> Self
    //    where F: 'static + Fn(&OwnProfile) -> AsyncResult<(), Error>
    //        { Self{ own_profile : Rc::new( RefCell::new(own_profile) ),
    //                signer, profile_repo, home_connector, handle, on_updated: Rc::new(on_updated),
    //                relations: Default::default(), session_cache: Default::default() } }

    pub fn connect_home(&self, home_profile_id: &ProfileId) -> AsyncResult<Rc<Home>, Error> {
        Self::connect_home2(
            home_profile_id,
            self.profile_repo.clone(),
            self.home_connector.clone(),
            self.signer.clone(),
        )
    }

    fn connect_home2(
        home_profile_id: &ProfileId,
        prof_repo: Rc<ProfileExplorer>,
        connector: Rc<HomeConnector>,
        signer: Rc<Signer>,
    ) -> AsyncResult<Rc<Home>, Error> {
        let home_conn_fut = prof_repo
            .fetch(home_profile_id)
            .inspect(move |home_profile| {
                debug!("Finished loading details for home {}", home_profile.id())
            })
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then(move |home_profile| {
                connector
                    .connect_to_home(&home_profile, signer)
                    .map_err(|err| err.context(ErrorKind::ConnectionToHomeFailed).into())
            });

        Box::new(home_conn_fut)
    }

    pub fn login_home(&self, home_profile_id: ProfileId) -> AsyncResult<Rc<MyHomeSession>, Error> {
        if let Some(ref session_rc) = self.session_cache.borrow().get(&home_profile_id) {
            return Box::new(Ok(Rc::clone(session_rc)).into_future());
        }

        let home_id = home_profile_id.clone();
        let my_profile_id = self.signer.profile_id().to_owned();
        let session_cache = self.session_cache.clone();
        let login_fut = self.profile_repo.fetch(&my_profile_id)
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then( |profile|
            {
                match profile.as_persona()
                {
                    Some(persona) => persona.homes.iter()
                        .filter(move |home_proof|
                            home_proof.peer_id(&my_profile_id)
                                .and_then(|peer_id|
                                    if *peer_id == home_id { Ok(true) }
                                    else { Err(mercury_home_protocol::error::ErrorKind::PeerIdRetreivalFailed.into()) }
                                )
                                .is_ok()
                        )
                        .map( |home_proof| home_proof.to_owned() )
                        .nth(0)
                        .ok_or(ErrorKind::HomeProofNotFound.into()),

                    None => Err(ErrorKind::PersonaProfileExpected.into())
                }
            } )
            .and_then(
            {
                let profile_repo_clone = self.profile_repo.clone();
                let home_connector_clone = self.home_connector.clone();
                let signer_clone = self.signer.clone();
                let handle = self.handle.clone();
                let handle2 = self.handle.clone();
                let relations_weak = Rc::downgrade(&self.relations);
                move |home_proof| {
                    Self::connect_home2(&home_profile_id, profile_repo_clone, home_connector_clone, signer_clone)
                        .and_then( move |home| {
                            home.login(&home_proof)
                                .map_err( |err| err.context(ErrorKind::LoginFailed).into() )
                                .map( move |session| MyHomeSessionImpl::new(session, handle) )
                                .inspect( move |my_session|
                                {
                                    // TODO this allows initiating several fill attempts in parallel
                                    //      until first one succeeds, last one wins by overwriting.
                                    //      Is this acceptable?
                                    session_cache.borrow_mut().insert( home_profile_id.to_owned(), my_session.clone() );
                                    Self::start_event_handler( relations_weak, my_session.clone(), &handle2 )
                                } )
                        } )
                }
            });
        Box::new(login_fut)
    }

    pub fn any_home_of(&self, profile: &Profile) -> AsyncResult<(RelationProof, Rc<Home>), Error> {
        MyProfileImpl::any_home_of2(
            profile,
            self.profile_repo.clone(),
            self.home_connector.clone(),
            self.signer.clone(),
        )
    }

    fn any_home_of2(
        profile: &Profile,
        prof_repo: Rc<ProfileExplorer>,
        connector: Rc<HomeConnector>,
        signer: Rc<Signer>,
    ) -> AsyncResult<(RelationProof, Rc<Home>), Error> {
        let homes = match profile.as_persona() {
            Some(ref persona) => persona.homes.clone(),
            None => return Box::new(future::err(ErrorKind::PersonaProfileExpected.into())),
        };

        let home_conn_futs = homes
            .iter()
            .map(move |home_proof| {
                let prof_repo = prof_repo.clone();
                let connector = connector.clone();
                let proof = home_proof.to_owned();
                match home_proof.peer_id(&profile.id()) {
                    Ok(ref home_id) => {
                        debug!("Scheduling connect_home2 for home id {}", home_id);
                        let conn_fut = Self::connect_home2(
                            home_id.to_owned(),
                            prof_repo,
                            connector,
                            signer.clone(),
                        )
                        .map(move |home| (proof, home));
                        Box::new(conn_fut) as AsyncResult<_, Error>
                    }
                    Err(e) => Box::new(future::err(e.context(ErrorKind::FailedToGetPeerId).into())),
                }
            })
            .collect::<Vec<_>>();

        // NOTE needed because select_ok() panics for empty lists instead of simply returning an error
        if home_conn_futs.len() == 0 {
            return Box::new(future::err(ErrorKind::NoHomesFound.into()));
        }

        // Pick first successful home connection
        debug!("Try connecting to {} homes", home_conn_futs.len());
        let result = future::select_ok(home_conn_futs)
            .map(|(home_conn, _pending_conn_futs)| home_conn)
            .inspect(|_home_conn| debug!("Connected to first home, ignoring the rest"));
        Box::new(result)
    }

    fn on_new_relation(
        relations: Weak<RefCell<Vec<RelationProof>>>,
        rel_proof: RelationProof,
    ) -> AsyncResult<(), Error> {
        debug!("Storing new relation: {:?}", rel_proof);
        let relations_rc = match relations.upgrade() {
            Some(rc) => rc,
            None => {
                debug!("Received new relation to store, but Rc upgrade failed");
                return Box::new(Err(ErrorKind::ImplementationError.into()).into_future());
            }
        };

        relations_rc.borrow_mut().push(rel_proof);
        Box::new(Ok(()).into_future())
    }

    fn start_event_handler(
        relations: Weak<RefCell<Vec<RelationProof>>>,
        session: Rc<MyHomeSession>,
        handle: &reactor::Handle,
    ) {
        let events = session.events();

        debug!("Start processing events to store accepted pairing responses as relations");
        handle.spawn(
            events
                .for_each(move |event| {
                    debug!("Profile event handler got new event to match: {:?}", event);
                    match event {
                        ProfileEvent::PairingResponse(rel_proof) => {
                            debug!("Got pairing response, saving relation");
                            let not_fut = Self::on_new_relation(relations.clone(), rel_proof)
                                .map_err(|e| error!("Notification on new relation failed: {}", e));
                            Box::new(not_fut) as AsyncResult<_, _>
                        }
                        _ => Box::new(Ok(()).into_future()),
                    }
                })
                .then(|res| {
                    debug!(
                        "Profile event handler read all events from stream, stopping with: {:?}",
                        res
                    );
                    Ok(())
                }),
        );
    }
}

impl MyProfile for MyProfileImpl {
    //fn own_profile(&self) -> &OwnProfile { &self.own_profile.borrow() }
    fn signer(&self) -> &Signer {
        &*self.signer
    }

    fn relations(&self) -> Vec<RelationProof> {
        self.relations.borrow().clone()
    }

    fn relations_with_peer(
        &self,
        peer_id: &ProfileId,
        app_filter: Option<&ApplicationId>,
        relation_type_filter: Option<&str>,
    ) -> Vec<RelationProof> {
        let mut relations = self.relations();
        // debug!( "Checking {} relations according to filter options", relations.len() );
        // debug!( "  Relation type filter: {:?}", relation_type_filter );
        let relations = relations
            .drain(..)
            // .inspect( |rel| debug!("Checking relation {:?}", rel) )
            .filter(|proof| {
                proof
                    .peer_id(&self.signer().profile_id())
                    .map(|p_id| *p_id == *peer_id)
                    .unwrap_or(false)
            })
            // .inspect( |rel| debug!("Relation matched peer id filter {:?}", peer_id) )
            .filter(|proof| app_filter.map_or(true, |app_id| proof.accessible_by(app_id)))
            // .inspect( |rel| debug!("Relation matched app filter {:?}", app_filter) )
            .filter(|proof| relation_type_filter.map_or(true, |rel| proof.relation_type == rel))
            // .inspect( |rel| debug!("Relation matched relation type filter {:?}", relation_type_filter) )
            .collect();
        relations
    }

    fn homes(&self) -> AsyncResult<Vec<RelationProof>, Error> {
        let res = match self.own_profile.borrow().public_data().as_persona() {
            Some(ref persona) => Ok(persona.homes.clone()),
            None => Err(Error::from(ErrorKind::PersonaProfileExpected)),
        };
        Box::new(res.into_future())
    }

    fn join_home(
        &self,
        home_id: ProfileId,
        //invite: Option<HomeInvitation>,
    ) -> AsyncResult<(), Error> {
        let half_proof = RelationHalfProof::new(
            RelationProof::RELATION_TYPE_HOSTED_ON_HOME,
            &home_id,
            &*self.signer,
        );

        let own_profile_cell = self.own_profile.clone();
        let own_profile_dataclone = self.own_profile.borrow().to_owned();
        //let profile_repo = self.profile_repo.clone();
        let reg_fut = self
            .connect_home(&home_id)
            .and_then(move |home| {
                home.register(own_profile_dataclone, half_proof) //, invite)
                    .map_err(|(_own_prof, err)| err.context(ErrorKind::RegistrationFailed).into())
            })
            // TODO we should also notify the AdminSession here to update its profile_store
            //.and_then( |own_profile| ... )
            .map(move |own_profile| {
                own_profile_cell.replace(own_profile.clone());
                // TODO remove this after testing
                // profile_repo.set(own_profile.public_data());
            });
        Box::new(reg_fut)
    }

    fn leave_home(
        &self,
        home_id: ProfileId,
        newhome_id: Option<Profile>,
    ) -> AsyncResult<(), Error> {
        let unreg_fut = self.login_home(home_id)
            .map_err(|err| err.context(ErrorKind::LoginFailed).into())
            .and_then( move |my_session|
                my_session.session()
                    .unregister(newhome_id)
                    .map_err(|err| err.context(ErrorKind::DeregistrationFailed).into())
            )
            // TODO we should also notify the AdminSession here to update its profile_store
            // .and_then( || ... )
            ;
        Box::new(unreg_fut)
    }

    //    fn update(&self, home_id: ProfileId, own_prof: &OwnProfile) ->
    //        AsyncResult<(), Error>
    //    {
    //        let own_profile_clone = own_prof.clone();
    //        let upd_fut = self.login_home(home_id)
    //            .map_err(|err| err.context(ErrorKind::LoginFailed).into())
    //            .and_then( move |my_session|
    //                my_session.session()
    //                    .update(own_profile_clone)
    //                    .map_err(|err| err.context(ErrorKind::ProfileUpdateFailed).into())
    //            );
    //        Box::new(upd_fut)
    //    }

    fn initiate_relation(
        &self,
        relation_type: &str,
        with_profile_id: &ProfileId,
    ) -> AsyncResult<(), Error> {
        debug!("Trying to send pairing request to {}", with_profile_id);

        let profile_repo_clone = self.profile_repo.clone();
        let home_connector_clone = self.home_connector.clone();
        let signer_clone = self.signer.clone();
        let rel_type_clone = relation_type.to_owned();

        // let profile_fut = match pairing_url {
        //     Some(url) => self.profile_repo.resolve(url),
        //     None      => self.profile_repo.get(with_profile_id),
        // };

        let profile_fut = self.profile_repo.fetch(with_profile_id);

        let pair_fut = profile_fut
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then(move |profile| {
                //let half_proof = MyProfileImpl::new_half_proof(rel_type_clone.as_str(), &profile.id, signer_clone.clone() );
                let half_proof =
                    RelationHalfProof::new(&rel_type_clone, &profile.id(), &*signer_clone.clone());
                MyProfileImpl::any_home_of2(
                    &profile,
                    profile_repo_clone,
                    home_connector_clone,
                    signer_clone,
                )
                .and_then(move |(_home_proof, home)| {
                    debug!("Contacted home of target profile, sending pairing request");
                    home.pair_request(half_proof)
                        .map_err(|err| err.context(ErrorKind::PairRequestFailed).into())
                })
            });

        Box::new(pair_fut)
    }

    fn accept_relation(&self, half_proof: &RelationHalfProof) -> AsyncResult<RelationProof, Error> {
        debug!("Trying to send pairing response");

        if half_proof.peer_id != self.own_profile.borrow().id() {
            return Box::new(Err(ErrorKind::LookupFailed.into()).into_future());
        }

        let proof = match RelationProof::sign_remaining_half(&half_proof, self.signer()) {
            Ok(proof) => proof,
            Err(e) => {
                return Box::new(Err(e.context(ErrorKind::FailedToAuthorize).into()).into_future());
            }
        };

        let proof_clone = proof.clone();
        let pair_fut = self
            .profile_repo
            .fetch(&half_proof.signer_id)
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then({
                let profile_repo = self.profile_repo.clone();
                let connector = self.home_connector.clone();
                let signer = self.signer.clone();
                move |profile| Self::any_home_of2(&profile, profile_repo, connector, signer)
            })
            .and_then(move |(_home_proof, home)| {
                debug!("Contacted home of target profile, sending pairing response");
                home.pair_response(proof)
                    .map_err(|err| err.context(ErrorKind::PeerResponseFailed).into())
            })
            .and_then({
                let relations = Rc::downgrade(&self.relations);
                move |()| {
                    Self::on_new_relation(relations, proof_clone.clone()).map(move |()| proof_clone)
                }
            });

        Box::new(pair_fut)
    }

    fn call(
        &self,
        proof: RelationProof,
        app: ApplicationId,
        init_payload: AppMessageFrame,
        to_caller: Option<AppMsgSink>,
    ) -> AsyncResult<Option<AppMsgSink>, Error> {
        let peer_id = match proof.peer_id(&self.signer.profile_id()) {
            Ok(id) => id.to_owned(),
            Err(e) => {
                return Box::new(Err(e.context(ErrorKind::LookupFailed).into()).into_future())
            }
        };

        let profile_repo = self.profile_repo.clone();
        let home_connector = self.home_connector.clone();
        let signer = self.signer.clone();
        let call_fut = self
            .profile_repo
            .fetch(&peer_id)
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then(|profile| Self::any_home_of2(&profile, profile_repo, home_connector, signer))
            .and_then(move |(_home_proof, home)| {
                debug!("Connected to home, calling target profile");
                home.call(app, CallRequestDetails { relation: proof, init_payload, to_caller })
                    .map_err(|err| err.context(ErrorKind::CallFailed).into())
            });
        Box::new(call_fut)
    }

    // TODO this should try connecting to ALL of our homes, using our collect_results() future util function
    fn login(&self) -> AsyncResult<Rc<MyHomeSession>, Error> {
        if let Some(ref session_rc) = self.session_cache.borrow().values().next() {
            return Box::new(Ok(Rc::clone(session_rc)).into_future());
        }

        let my_profile_id = self.signer.profile_id().to_owned();
        let session_cache = self.session_cache.clone();
        let handle = self.handle.clone();
        let handle2 = self.handle.clone();
        let relations_weak = Rc::downgrade(&self.relations);
        let log_fut = self
            .profile_repo
            .fetch(&self.signer.profile_id())
            .map_err(|err| err.context(ErrorKind::LoginFailed).into())
            .and_then({
                let profile_repo_clone = self.profile_repo.clone();
                let home_conn_clone = self.home_connector.clone();
                let signer_clone = self.signer.clone();
                move |profile| {
                    debug!("Client profile was loaded for login, connecting home");
                    MyProfileImpl::any_home_of2(
                        &profile,
                        profile_repo_clone,
                        home_conn_clone,
                        signer_clone,
                    )
                }
            })
            .and_then(move |(home_proof, home)| {
                debug!("Home connection established, logging in");
                let home_id = match home_proof.peer_id(&my_profile_id) {
                    Ok(id) => id.to_owned(),
                    Err(e) => {
                        return Box::new(
                            Err(e.context(ErrorKind::FailedToAuthorize).into()).into_future(),
                        ) as AsyncResult<_, _>;
                    }
                };
                let login_fut = home
                    .login(&home_proof)
                    .map_err(|err| err.context(ErrorKind::LoginFailed).into())
                    .map(move |session| MyHomeSessionImpl::new(session, handle))
                    .inspect(move |my_session| {
                        // TODO this allows initiating several fill attempts in parallel
                        //      until first one succeeds, last one wins by overwriting.
                        //      Is this acceptable?
                        session_cache.borrow_mut().insert(home_id, my_session.clone());
                        Self::start_event_handler(relations_weak, my_session.clone(), &handle2)
                    });
                Box::new(login_fut)
            });

        Box::new(log_fut)
    }
}

impl Drop for MyProfileImpl {
    fn drop(&mut self) {
        debug!("MyProfile was dropped");
    }
}

pub struct MyHomeSessionImpl {
    session: Rc<HomeSession>,
    event_listeners: Rc<RefCell<Vec<EventSink>>>,
}

impl MyHomeSessionImpl {
    fn new(session: Rc<HomeSession>, handle: reactor::Handle) -> Rc<MyHomeSession> {
        let this = Rc::new(Self { session, event_listeners: Default::default() });

        debug!("Created MyHomeSession, start forwarding profile events to listeners");
        let listeners = Rc::downgrade(&this.event_listeners);
        handle.spawn(this.session.events().for_each(move |event| {
            debug!("Received event {:?}, dispatching", event);
            Self::forward_event_safe(listeners.clone(), event)
        }));

        this
    }

    pub fn add_listener(event_listeners: Rc<RefCell<Vec<EventSink>>>, listener: EventSink) {
        debug!("Adding new event listener to session");
        event_listeners.borrow_mut().push(listener);
    }

    // Call forward event with safety measures on: respect a dropped service and remote errors sent by the home
    fn forward_event_safe(
        event_listeners_weak: Weak<RefCell<Vec<EventSink>>>,
        event_res: Result<ProfileEvent, String>,
    ) -> AsyncResult<(), ()> {
        // Get strong Rc from Weak, stop forwarding if Rc is already dropped
        let event_listeners_rc = match event_listeners_weak.upgrade() {
            Some(listeners) => listeners,
            None => {
                debug!("Stop event forwarding for profile after underlying session was dropped");
                return Box::new(Err(()).into_future()); // NOTE error only to break for_each, otherwise normal
            }
        };

        // Try unwrapping and forwarding event, stop forwarding if received remote error
        match event_res {
            Ok(event) => {
                let listeners = event_listeners_rc.replace(Vec::new());
                debug!("Notifying {} listeners on incoming event", listeners.len());
                let fwd_fut =
                    Self::forward_event(listeners, event).map(move |successful_listeners| {
                        let mut listeners = event_listeners_rc.borrow_mut();
                        debug!(
                            "{} listeners were notified, detected {} new listeners meanwhile",
                            successful_listeners.len(),
                            listeners.len()
                        );
                        listeners.extend(successful_listeners); // Use extend instead of assignment to keep listeners added meanwhile
                    });
                Box::new(fwd_fut) as AsyncResult<(), ()>
            }
            Err(e) => {
                warn!("Remote error listening to profile events, stopping listeners: {}", e);
                Box::new(Err(()).into_future())
            }
        }
    }

    // Notify all registered listeners of an incoming profile event,
    // removing failing (i.e. dropped) listeners from the list
    fn forward_event(
        mut event_listeners: Vec<EventSink>,
        event: ProfileEvent,
    ) -> AsyncResult<Vec<EventSink>, ()> {
        // Create tasks (futures) of sending an item to each listener
        let send_futs = event_listeners.drain(..).map(|listener| listener.send(event.clone()));

        // Collect successful senders, drop failing ones
        let fwd_fut = fut::collect_results(send_futs)
            .map(|mut send_results| send_results.drain(..).filter_map(|res| res.ok()).collect());

        Box::new(fwd_fut)
    }
}

impl Drop for MyHomeSessionImpl {
    fn drop(&mut self) {
        debug!("MyHomeSessionImpl was dropped");
    }
}

impl MyHomeSession for MyHomeSessionImpl {
    fn session(&self) -> Rc<HomeSession> {
        self.session.clone()
    }

    fn events(&self) -> EventStream {
        let (listener, events) = mpsc::channel(CHANNEL_CAPACITY);
        Self::add_listener(self.event_listeners.clone(), listener);
        events
    }
}
