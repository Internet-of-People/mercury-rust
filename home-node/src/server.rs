use std::collections::HashMap;
use std::time::Duration;
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use async_trait::async_trait;
use failure::{Fail, Fallible};
use futures::{stream, TryFutureExt};
use log::*;
use tokio::future::FutureExt;
use tokio::prelude::*;
use tokio::sync::{mpsc, oneshot};

use claims::model::Link;
use mercury_home_protocol::api::AsyncSink; // TODO this should normally work with protocol::*, why is this needed?
use mercury_home_protocol::error::*;
use mercury_home_protocol::*;
use mercury_storage::asynch::KeyValueStore;

// TODO this should come from user configuration with a reasonable default value close to this
const CFG_CALL_ANSWER_TIMEOUT: Duration = Duration::from_secs(30);

pub struct HomeServer {
    validator: Rc<dyn Validator>,
    public_profile_dht: Rc<RefCell<dyn DistributedPublicProfileRepository>>,
    private_backup_db: Rc<RefCell<dyn PrivateProfileRepository>>,
    host_relations_db: Rc<RefCell<dyn KeyValueStore<ProfileId, RelationProof>>>,
    sessions: Rc<RefCell<HashMap<ProfileId, Weak<HomeSessionServer>>>>,
}

impl HomeServer {
    pub fn new(
        validator: Rc<dyn Validator>,
        public_dht: Rc<RefCell<dyn DistributedPublicProfileRepository>>,
        private_db: Rc<RefCell<dyn PrivateProfileRepository>>,
        host_relations_db: Rc<RefCell<dyn KeyValueStore<ProfileId, RelationProof>>>,
    ) -> Self {
        Self {
            validator,
            public_profile_dht: public_dht,
            private_backup_db: private_db,
            host_relations_db,
            sessions: Default::default(),
        }
    }
}

pub struct HomeConnectionServer {
    server: Rc<HomeServer>, // TODO consider if we should have a RefCell<> for mutability here
    context: Rc<PeerContext>,
}

impl HomeConnectionServer {
    pub fn new(context: Rc<PeerContext>, server: Rc<HomeServer>) -> Result<Self, Error> {
        context
            .validate(&*server.validator)
            .map_err(|err| err.context(ErrorKind::ContextValidationFailed))?;
        Ok(Self { context, server })
    }

    /// Returns Error if the profile is not hosted on this home server
    /// Returns None if the profile is not online
    async fn get_live_session(
        server: Rc<HomeServer>,
        to_profile: ProfileId,
    ) -> Fallible<Option<Rc<HomeSessionServer>>> {
        // Check if this profile is hosted on this server
        // TODO we probably should check the hosted_profile_db here instead
        let _profile_data = server.private_backup_db.borrow().get(&to_profile).await?;
        // Separate variable needed, see https://stackoverflow.com/questions/50391668/running-asynchronous-mutable-operations-with-rust-futures
        let sessions = server.sessions.borrow();
        // If hosted here, check if profile is in reach with an online session
        let session_weak_opt = sessions.get(&to_profile);
        let session_rc_opt = session_weak_opt.and_then(|weak| weak.upgrade());

        Ok(session_rc_opt)
    }

    async fn push_event(
        server: Rc<HomeServer>,
        to_profile: ProfileId,
        event: ProfileEvent,
    ) -> Fallible<()> {
        debug!("Dispatching event {:?} to session of profile {}", event, to_profile);
        if let Some(session) = Self::get_live_session(server, to_profile).await? {
            // TODO if push to session fails, consider just dropping the session
            //      (is anything manual needed using weak pointers?) and requiring a reconnect
            session.push_event(event).await
        } else {
            Ok(())
            // TODO save event into persistent storage and delegate it when profile is online again
        }
    }

    async fn push_call(
        server: Rc<HomeServer>,
        to_profile: ProfileId,
        to_app: ApplicationId,
        call: Box<dyn IncomingCall>,
    ) -> Fallible<()> {
        let session_rc_opt = Self::get_live_session(server, to_profile).await?;
        match session_rc_opt {
            Some(session) => {
                // TODO if push to session fails, consider just dropping the session
                //      (is anything manual needed using weak pointers?) and requiring a reconnect
                session.push_call(to_app, call).await
            }
            // TODO save event into persistent storage and delegate it when profile is online again
            None => Ok(()),
        }
    }
}

#[async_trait(?Send)]
impl ProfileExplorer for HomeConnectionServer {
    async fn fetch(&self, id: &ProfileId) -> Fallible<Profile> {
        self.server
            .public_profile_dht
            .borrow()
            .get_public(id)
            .await
            .map_err(|e| e.context(ErrorKind::DhtLookupFailed).into())
    }

    async fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!()
    }
}

#[async_trait(?Send)]
impl Home for HomeConnectionServer {
    async fn claim(&self, profile_id: ProfileId) -> Fallible<RelationProof> {
        if profile_id != self.context.peer_id() {
            return Err(ErrorKind::FailedToClaimProfile.into());
        }
        self.server.host_relations_db.borrow().get(profile_id).await
    }

    async fn register(
        &self,
        half_proof: &RelationHalfProof,
        //_invite: Option<HomeInvitation>,
    ) -> Fallible<RelationProof> {
        if &half_proof.signer_id != &self.context.peer_id() {
            return Err(ErrorKind::SignerMismatch.into());
        }
        if &half_proof.signer_pubkey != &self.context.peer_pubkey() {
            return Err(ErrorKind::PublicKeyMismatch.into());
        }

        trace!(
            "Request was sent for home_id: {:?}, this should be me, i.e. match my id: {:?}",
            &half_proof.peer_id,
            &self.context.my_signer().profile_id()
        );
        if &half_proof.peer_id != self.context.my_signer().profile_id() {
            return Err(ErrorKind::HomeIdMismatch.into());
        }

        if &half_proof.relation_type != RelationProof::RELATION_TYPE_HOSTED_ON_HOME {
            return Err(ErrorKind::RelationTypeMismatch.into());
        }

        self.server.validator.validate_half_proof(half_proof, &self.context.peer_pubkey())?;

        let host_proof = RelationProof::sign_remaining_half(half_proof, self.context.my_signer())?;

        let profile_id = &half_proof.signer_id;
        let existing_proof_res =
            self.server.host_relations_db.borrow().get(profile_id.to_owned()).await;
        match existing_proof_res {
            Ok(_existing_proof) => {
                debug!("Profile was already registered");
                return Err(ErrorKind::AlreadyRegistered.into());
            }
            // TODO only errors like NotFound should be accepted here but other (e.g. I/O) errors should be delegated
            Err(_e) => {}
        }

        // Store private profile info in local storage only (e.g. SQL)
        debug!("Saving private profile info into local storage");
        self.server
            .host_relations_db
            .borrow_mut()
            .set(profile_id.to_owned(), host_proof.clone())
            .await?;

        Ok(host_proof)
    }

    async fn login(&self, proof_of_home: &RelationProof) -> Fallible<Rc<dyn HomeSession>> {
        if &proof_of_home.relation_type != RelationProof::RELATION_TYPE_HOSTED_ON_HOME {
            return Err(ErrorKind::RelationTypeMismatch.into());
        }
        let home_id = self.context.my_signer().profile_id();
        let home_pk = &self.context.my_signer().public_key();
        let peer_id = proof_of_home.peer_id(home_id)?;
        let peer_pk = proof_of_home.peer_pub_key(home_id)?;
        if peer_id != &self.context.peer_id() {
            return Err(ErrorKind::ProfileMismatch.into());
        }
        if peer_pk != &self.context.peer_pubkey() {
            return Err(ErrorKind::PublicKeyMismatch.into());
        }
        self.server.validator.validate_profile_auth(peer_pk, peer_id)?;
        self.server.validator.validate_relation_proof(
            &proof_of_home,
            peer_id,
            peer_pk,
            home_id,
            home_pk,
        )?;

        // TODO we probably should check the hosted_profile_db here instead
        let _own_profile = self.server.private_backup_db.borrow().get(peer_id).await?;

        let session =
            Rc::new(HomeSessionServer::new(self.context.to_owned(), self.server.to_owned()));
        self.server
            .sessions
            .borrow_mut()
            .entry(peer_id.to_owned())
            .or_insert(Rc::downgrade(&session));

        Ok(session as Rc<dyn HomeSession>)
    }

    async fn pair_request(&self, half_proof: &RelationHalfProof) -> Fallible<()> {
        if half_proof.signer_id != self.context.peer_id() {
            return Err(ErrorKind::ProfileMismatch.into());
        }

        self.server.validator.validate_half_proof(&half_proof, &self.context.peer_pubkey())?;

        let to_profile = half_proof.peer_id.to_owned();
        let event = ProfileEvent::PairingRequest(half_proof.to_owned());
        Self::push_event(self.server.clone(), to_profile, event).await
    }

    async fn pair_response(&self, relation: &RelationProof) -> Fallible<()> {
        let (peer_id, hosted_id) = self.validate_access(&relation).await?;
        debug!("Got pairing response from {} to {}", &peer_id, &hosted_id);

        Self::push_event(
            self.server.clone(),
            hosted_id.to_owned(),
            ProfileEvent::PairingResponse(relation.to_owned()),
        )
        .await?;
        Ok(())
    }

    async fn call(
        &self,
        app: ApplicationId,
        call_req: &CallRequestDetails,
    ) -> Fallible<Option<AppMsgSink>> {
        let (peer_id, hosted_id) = self.validate_access(&call_req.relation).await?;
        if &hosted_id == &self.context.peer_id() {
            // must not call self through home
            return Fallible::Err(ErrorKind::CallFailed.into());
        }
        debug!("Got call from {} to {}", &peer_id, &hosted_id);

        let (send, recv) = oneshot::channel::<Option<AppMsgSink>>();
        let call = Box::new(Call::new(call_req.to_owned(), send));
        Self::push_call(self.server.to_owned(), hosted_id.to_owned(), app, call).await?;
        //            .map_err(|err| err.context(ErrorKind::CallFailed).into())?;

        let answer_fut = async {
            match recv.await {
                Ok(v) => Fallible::Ok(v),
                Err(e) => {
                    Fallible::Err(failure::Error::from(e.context(ErrorKind::FailedToReadResponse)))
                }
            }
        };

        // Wait for answer with specified timeout
        let timeout_fut = answer_fut.timeout(CFG_CALL_ANSWER_TIMEOUT).map_err(|e| {
            info!("No response for call until timeout: {}", e);
            failure::Error::from(e.context(ErrorKind::FailedToReadResponse))
        });

        let answer: Option<AppMsgSink> = timeout_fut.await??; // Timeout::Output is Result<Result<_, _>,_>
        Ok(answer)
    }
}

struct Call {
    request: CallRequestDetails,
    sender: oneshot::Sender<Option<AppMsgSink>>,
}

impl Call {
    pub fn new(request: CallRequestDetails, sender: oneshot::Sender<Option<AppMsgSink>>) -> Self {
        Self { request, sender }
    }
}

impl IncomingCall for Call {
    fn request_details(&self) -> &CallRequestDetails {
        &self.request
    }
    fn answer(self: Box<Self>, to_callee: Option<AppMsgSink>) -> CallRequestDetails {
        // NOTE needed to dereference Box because otherwise the whole self is moved at its first dereference
        let this = *self;
        if let Err(_e) = this.sender.send(to_callee) {
            // TODO We should at least log the error here.
            //      To solve this better, the function probably should return a Result<T,E> instead of T.
        }
        this.request
    }
}

enum ServerSink<T, E> {
    // TODO message buffer should be persistent on the long run
    Buffer(Vec<Result<T, E>>), // Temporary buffer, sink is not initialized
    Sender(AsyncSink<T, E>), // Initialized sink end of channel, user is listening on the other half
}

impl<T, E> Default for ServerSink<T, E> {
    fn default() -> Self {
        Self::Buffer(Default::default())
    }
}

type ProfileEventSink = ServerSink<ProfileEvent, String>;
type CallSink = ServerSink<Box<dyn IncomingCall>, String>;

pub struct HomeSessionServer {
    // TODO consider using Weak<Ptrs> instead of Rc<Ptrs> if a closed Home connection cannot
    //      drop all related sessions automatically
    context: Rc<PeerContext>,
    server: Rc<HomeServer>,
    events: Rc<RefCell<ProfileEventSink>>,
    apps: Rc<RefCell<HashMap<ApplicationId, CallSink>>>, // {appId->sender<call>}
}

impl HomeSessionServer {
    // TODO consider if validating the context is needed here, e.g. as an assert()
    pub fn new(context: Rc<PeerContext>, server: Rc<HomeServer>) -> Self {
        Self { context, server, events: Default::default(), apps: Default::default() }
    }

    async fn push_event(&self, event: ProfileEvent) -> Fallible<()> {
        debug!("Session with {} got event dispatched: {:?}", &self.context.peer_id(), &event);
        match *self.events.borrow_mut() {
            ServerSink::Buffer(ref mut bufvec) => {
                debug!("No event channel is available on the client side, buffering event");
                bufvec.push(Ok(event)); // TODO consider size constraints
                Ok(())
            }
            ServerSink::Sender(ref mut sender) => {
                debug!("Event channel for active client was found, sending event there");
                sender.send(Ok(event)).await.map_err_fail(ErrorKind::FailedToSend)
                // TODO if call dispatch fails we probably should replace the sender with a buffer
            }
        }
    }

    async fn push_call(&self, app: ApplicationId, call: Box<dyn IncomingCall>) -> Fallible<()> {
        debug!(
            "Session with {} dispatched call with relation: {:?}",
            &self.context.peer_id(),
            &call.request_details().relation
        );
        let mut apps = self.apps.borrow_mut();
        let sink = apps.entry(app).or_insert(ServerSink::Buffer(Vec::new()));
        match *sink {
            ServerSink::Buffer(ref mut bufvec) => {
                bufvec.push(Ok(call)); // TODO consider size constraints
                Ok(())
            }
            ServerSink::Sender(ref mut sender) => {
                // TODO if call dispatch fails we probably should replace the sender with a buffer
                sender.send(Ok(call)).await.map_err_fail(ErrorKind::CallFailed)
            }
        }
    }
}

impl Drop for HomeSessionServer {
    fn drop(&mut self) {
        let peer_id = self.context.peer_id();
        debug!("dropping session {}", peer_id);
        self.server.sessions.borrow_mut().remove(&peer_id);
    }
}

#[async_trait(?Send)]
impl HomeSession for HomeSessionServer {
    async fn backup(&self, own_prof: OwnProfile) -> Fallible<()> {
        if own_prof.id() != self.context.peer_id() {
            return Err(ErrorKind::ProfileMismatch.into());
        }
        if own_prof.public_key() != self.context.peer_pubkey() {
            return Err(ErrorKind::PublicKeyMismatch.into());
        }

        let _own_prof_orig = self.server.private_backup_db.borrow().get(&own_prof.id()).await?;
        self.server.public_profile_dht.borrow_mut().set_public(own_prof.public_data()).await?;
        self.server.private_backup_db.borrow_mut().set(own_prof).await?;

        Ok(())
    }

    async fn restore(&self) -> Fallible<OwnProfile> {
        let own_prof = self.server.private_backup_db.borrow().get(&self.context.peer_id()).await?;

        Ok(own_prof)
    }

    // TODO is the ID of the new home enough here or do we need the whole profile?
    // TODO new_home should be stored and some special redirect to new home should be sent when someone looking for the profile
    async fn unregister(&self, _new_home: Option<Profile>) -> Fallible<()> {
        let profile_id = self.context.peer_id();
        let profile_key = self.context.peer_pubkey();

        // TODO how to delete profile from self.server.hosted_profiles_db? We'll probably need a remove operation
        // Drop session reference from server
        self.server.sessions.borrow_mut().remove(&profile_id);

        // TODO force close/drop session connection after successful unregister().
        //      Ideally self would be consumed here, but that'd require binding to self: Box<Self> or Rc<Self> to compile within a trait.
        self.server.public_profile_dht.borrow_mut().clear_public_local(&profile_key).await?;
        self.server.private_backup_db.borrow_mut().clear(&profile_key).await?;

        Ok(())
    }

    // TODO investigate if race condition is possible, e.g. an event was sent out to the old_sender,
    //      and a repeated events() call is received. In this case, can we be sure that the event
    //      has been processed via the old_sender?
    fn events(&self) -> AsyncStream<ProfileEvent, String> {
        let (mut sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);

        // Set up events with the new channel and check the old event sink
        match self.events.replace(ServerSink::Sender(sender.clone())) {
            // We already had another channel properly set up
            ServerSink::Sender(mut old_sender) => {
                // NOTE consuming the events stream multiple times is likely a client implementation error
                tokio::runtime::current_thread::spawn(async move {
                    let _res = old_sender.send( Err( "WARNING: Repeated call of HomeSession::events() detected, this channel is dropped, using the new one".to_owned() ) ).await;
                });
            }
            // The client was not listening to events so far, the channel is brand new
            ServerSink::Buffer(mut msg_vec) => {
                // Send all collected messages from buffer as we now finally have a channel to the user
                // TODO use persistent storage for events when profile is offline and delegate them here
                tokio::runtime::current_thread::spawn(async move {
                    let _res = sender.send_all(&mut stream::iter(msg_vec.drain(..))).await;
                });
            }
        }

        receiver
    }

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> AsyncStream<Box<dyn IncomingCall>, String> {
        let (mut sender, receiver) = mpsc::channel(CHANNEL_CAPACITY);
        let apps = self.apps.to_owned();
        let app = app.to_owned();
        let send_missed_msgs_fut = async move {
            let mut old_stream = apps.borrow_mut().insert(app, ServerSink::Sender(sender.clone()));
            match old_stream {
                Some(ServerSink::Sender(ref mut old_sender)) => {
                    // NOTE consuming the calls stream multiple times is likely a client implementation error
                    let _res = old_sender.send( Err( "WARNING: Repeated call of HomeSession::checkin_app() detected, this channel is dropped, using the new one".to_owned() )).await;
                }
                Some(ServerSink::Buffer(ref mut call_vec)) => {
                    // Send all collected calls from buffer as we now finally have a channel to the app
                    // TODO use persistent storage for calls when profile is offline and delegate them here
                    let _res = sender.send_all(&mut stream::iter(call_vec.drain(..))).await;
                }
                None => {}
            }
        };
        tokio::runtime::current_thread::spawn(send_missed_msgs_fut);

        // TODO how to detect dropped stream and remove the sink from the session?
        receiver
    }

    // TODO consider removing this after testing
    async fn ping(&self, txt: &str) -> Fallible<String> {
        debug!("Ping received `{}`, sending it back", txt);
        Ok(txt.to_owned())
    }
}

impl HomeConnectionServer {
    async fn validate_access(&self, relation: &RelationProof) -> Fallible<(ProfileId, ProfileId)> {
        let peer_id = self.context.peer_id();
        let peer_pk = &self.context.peer_pubkey();
        let hosted_id = relation
            .peer_id(&peer_id)
            .map(|id| id.to_owned())
            .map_err(|err| Error::from(err.context(ErrorKind::ProfileMismatch)))?;

        // We need to look up the public key to be able to validate the proof
        let hosted_profile = self
            .server
            // TODO we probably should check the hosted_profile_db here instead
            .private_backup_db
            .borrow()
            .get(&hosted_id)
            .await
            .map_err(|err| Error::from(err.context(ErrorKind::PeerNotHostedHere)))?;
        debug_assert_eq!(&hosted_profile.id(), &hosted_id);
        let hosted_pk = &hosted_profile.public_key();
        debug_assert!(hosted_pk.validate_id(&hosted_id));
        self.server
            .validator
            .validate_relation_proof(&relation, &peer_id, peer_pk, &hosted_id, hosted_pk)
            .map_err(|err| Error::from(err.context(ErrorKind::InvalidRelationProof)))?;

        Ok((peer_id, hosted_id))
    }
}
