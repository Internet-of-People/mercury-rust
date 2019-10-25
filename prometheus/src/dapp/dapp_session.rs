use std::sync::{Arc, RwLock};

use futures::{Future, Stream};

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
pub trait Relation {
    fn proof(&self) -> &RelationProof;
    // TODO fn send(&self, message: &MessageContent) -> AsyncFallible<()>;
    fn call(&self, init_payload: AppMessageFrame) -> AsyncFallible<DAppCall>;
}

pub enum DAppEvent {
    PairingResponse(Box<dyn Relation>),
    //TODO Message(...)
    Call(Box<dyn IncomingCall>), // TODO wrap IncomingCall so as call.answer() could return a DAppCall directly
}

pub trait DAppSession {
    fn dapp_id(&self) -> &ApplicationId;

    // After the session was initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> &ProfileId;

    //fn app_storage(&self) -> AsyncFallible<dyn KeyValueStore<String, String>>;

    fn relations(&self) -> AsyncFallible<Vec<Box<dyn Relation>>>;
    fn relation(&self, id: &ProfileId) -> AsyncFallible<Option<Box<dyn Relation>>>;
    fn initiate_relation(&self, with_profile: &ProfileId) -> AsyncFallible<()>;

    fn checkin(&self) -> AsyncFallible<Box<dyn Stream<Item = DAppEvent, Error = ()>>>;
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

impl DAppSession for DAppSessionImpl {
    fn dapp_id(&self) -> &ApplicationId {
        &self.dapp_id
    }

    fn selected_profile(&self) -> &ProfileId {
        &self.profile_id
    }

    //fn app_storage(&self) -> AsyncFallible<dyn KeyValueStore<String, String>> {
    //    unimplemented!()
    //}

    fn relations(&self) -> AsyncFallible<Vec<Box<dyn Relation>>> {
        unimplemented!()
    }

    fn relation(&self, _id: &ProfileId) -> AsyncFallible<Option<Box<dyn Relation>>> {
        unimplemented!()
    }

    fn initiate_relation(&self, _with_profile: &ProfileId) -> AsyncFallible<()> {
        unimplemented!()
    }

    fn checkin(&self) -> AsyncFallible<Box<dyn Stream<Item = DAppEvent, Error = ()>>> {
        unimplemented!()
    }
}

pub trait DAppSessionService {
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn dapp_session(&self, app: ApplicationId) -> AsyncFallible<Arc<dyn DAppSession>>;
}

pub struct DAppSessionServiceImpl {
    interactor: Arc<RwLock<dyn UserInteractor + Send + Sync>>,
}

impl DAppSessionServiceImpl {
    pub fn new(interactor: Arc<RwLock<dyn UserInteractor + Send + Sync>>) -> Self {
        Self { interactor }
    }
}

impl DAppSessionService for DAppSessionServiceImpl {
    fn dapp_session(&self, app: ApplicationId) -> AsyncFallible<Arc<dyn DAppSession>> {
        let interactor = match self.interactor.try_read() {
            Ok(interactor) => interactor,
            Err(e) => {
                error!("BUG: failed to lock user interactor: {}", e);
                unreachable!()
            }
        };
        let session_fut = interactor.select_profile().map(move |profile| {
            Arc::new(DAppSessionImpl::new(app, profile)) as Arc<dyn DAppSession>
        });
        Box::new(session_fut)
    }
}
