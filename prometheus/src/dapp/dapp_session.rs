use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use failure::Fallible;
use futures::Stream;

use crate::dapp::user_interactor::UserInteractor;
use crate::*;
use claims::model::*;
use mercury_home_protocol::{
    AppMessageFrame, AppMsgSink, AppMsgStream, ApplicationId, IncomingCall, RelationProof,
};

pub struct DAppCall {
    pub outgoing: AppMsgSink,
    pub incoming: AppMsgStream,
}

// - if messaging dApp still does not have access rights to sender profile then request access
//   (in first iteration automatically approve it)
// - instantiate some kind of client to a Home node, similarly as done in Connect
// - potentially initiate pairing with profile if not done yet
// - send message via client to target profile
#[async_trait(?Send)]
pub trait Relation {
    fn proof(&self) -> &RelationProof;
    // TODO async fn send(&self, message: &MessageContent) -> Fallible<()>;
    async fn call(&self, init_payload: AppMessageFrame) -> Fallible<DAppCall>;
}

pub enum DAppEvent {
    PairingResponse(Box<dyn Relation>),
    //TODO Message(...)
    Call(Box<dyn IncomingCall>), // TODO wrap IncomingCall so as call.answer() could return a DAppCall directly
}

pub type DAppEventStream = Box<dyn Stream<Item = DAppEvent> + Unpin>;

#[async_trait]
pub trait DAppSession {
    fn dapp_id(&self) -> &ApplicationId;

    // After the session was initialized, the profile is selected and can be queried any time
    fn profile_id(&self) -> &ProfileId;

    //fn app_storage(&self) -> AsyncFallible<dyn KeyValueStore<String, String>>;

    async fn relations(&self) -> Fallible<Vec<Box<dyn Relation>>>;
    async fn relation(&self, id: &ProfileId) -> Fallible<Option<Box<dyn Relation>>>;
    async fn initiate_relation(&self, with_profile: &ProfileId) -> Fallible<()>;

    async fn checkin(&self) -> Fallible<DAppEventStream>;
}

pub struct DAppSessionImpl {
    dapp_id: ApplicationId,
    profile_id: ProfileId,
}

impl DAppSessionImpl {
    pub fn new(dapp_id: ApplicationId, profile_id: ProfileId) -> Self {
        Self { profile_id, dapp_id }
    }
}

#[async_trait]
impl DAppSession for DAppSessionImpl {
    fn dapp_id(&self) -> &ApplicationId {
        &self.dapp_id
    }

    fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }

    //fn app_storage(&self) -> AsyncFallible<dyn KeyValueStore<String, String>> {
    //    unimplemented!()
    //}

    async fn relations(&self) -> Fallible<Vec<Box<dyn Relation>>> {
        unimplemented!()
    }

    async fn relation(&self, _id: &ProfileId) -> Fallible<Option<Box<dyn Relation>>> {
        unimplemented!()
    }

    async fn initiate_relation(&self, _with_profile: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }

    async fn checkin(&self) -> Fallible<DAppEventStream> {
        unimplemented!()
    }
}

#[async_trait(?Send)]
pub trait DAppSessionService {
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    async fn dapp_session(&self, app: ApplicationId) -> Fallible<Arc<dyn DAppSession>>;
}

pub struct DAppSessionServiceImpl {
    interactor: Arc<RwLock<dyn UserInteractor + Send + Sync>>,
}

impl DAppSessionServiceImpl {
    pub fn new(interactor: Arc<RwLock<dyn UserInteractor + Send + Sync>>) -> Self {
        Self { interactor }
    }
}

#[async_trait(?Send)]
impl DAppSessionService for DAppSessionServiceImpl {
    async fn dapp_session(&self, app: ApplicationId) -> Fallible<Arc<dyn DAppSession>> {
        let interactor = match self.interactor.try_read() {
            Ok(interactor) => interactor,
            Err(e) => {
                error!("BUG: failed to lock user interactor: {}", e);
                unreachable!()
            }
        };
        let profile = interactor.select_profile().await?;
        let session = Arc::new(DAppSessionImpl::new(app, profile)) as Arc<dyn DAppSession>;
        Ok(session)
    }
}
