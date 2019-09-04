pub mod error;
pub mod profile;
pub use error::{Error, ErrorKind};
pub mod net;
pub use net::SimpleTcpHomeConnector;
pub mod jsonrpc;
pub mod sdk;
pub mod service;

use std::rc::Rc;

use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use mercury_home_protocol::*;
use mercury_storage::asynch::KeyValueStore;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppPermission(Vec<u8>);

pub trait Contact {
    fn proof(&self) -> &RelationProof;
    fn call(&self, init_payload: AppMessageFrame) -> AsyncResult<DAppCall, Error>;
}

pub struct DAppCall {
    pub outgoing: AppMsgSink,
    pub incoming: AppMsgStream,
}

//impl Drop for DAppCall
//    { fn drop(&mut self) { debug!("DAppCall was dropped"); } }

pub enum DAppEvent {
    PairingResponse(Box<dyn Contact>),
    Call(Box<dyn IncomingCall>), // TODO wrap IncomingCall so as call.answer() could return a DAppCall directly
}

pub trait DAppEndpoint {
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn dapp_session(
        &self,
        app: &ApplicationId,
        authorization: Option<DAppPermission>,
    ) -> AsyncResult<Rc<dyn DAppSession>, Error>;
}

// NOTE A specific DApp is logged in to the Connect Service with given details, e.g. a selected profile.
//      A DApp might have several sessions, e.g. running in the name of multiple profiles.
pub trait DAppSession {
    // After the session was initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> ProfileId;

    // TODO merge these two operations using an optional profile argument
    fn contacts(&self) -> AsyncResult<Vec<Box<dyn Contact>>, Error>;
    fn contacts_with_profile(
        &self,
        profile: &ProfileId,
        relation_type: Option<&str>,
    ) -> AsyncResult<Vec<Box<dyn Contact>>, Error>;
    fn initiate_contact(&self, with_profile: &ProfileId) -> AsyncResult<(), Error>;

    fn app_storage(&self) -> AsyncResult<dyn KeyValueStore<String, String>, Error>;

    fn checkin(&self) -> AsyncResult<Box<dyn Stream<Item = DAppEvent, Error = ()>>, Error>;
}
