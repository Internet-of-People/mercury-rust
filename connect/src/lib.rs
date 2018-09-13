extern crate futures;
#[macro_use]
extern crate log;
extern crate mercury_home_protocol;
extern crate mercury_storage;
extern crate multiaddr;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate tokio_core;
extern crate tokio_io;
#[macro_use]
extern crate failure;



pub mod client;
pub mod error;
pub mod net;
pub use net::SimpleTcpHomeConnector;

pub mod sdk;
pub mod service;

pub mod simple_profile_repo;
pub use simple_profile_repo::SimpleProfileRepo;



use std::rc::Rc;

use futures::prelude::*;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::error::*;



// TODO maybe this should be transformed to store a relationproof with an operation like
//      fn profile(&self) -> Box<Future<Item=Profile,Error=SomeError>>
//      cache profile after fetched in something like an Option<RefCell<Profile>>
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Relation
{
    pub proof:      RelationProof,
// TODO consider transforming Profile to Option<WeakRef<Profile>> with an operation like
//      fn peer(&self) -> Box<Future<Item=Profile,Error=SomeError>>
//      which could return a cache profile value immediately or load it if not present yet
    pub peer:       Profile,
}

impl Relation
{
    pub fn new(peer: &Profile, proof: &RelationProof) -> Self
        { Self { peer: peer.clone(), proof: proof.clone() } }

//    pub fn call(&self, init_payload: AppMessageFrame,
//                to_caller: Option<AppMsgSink>) ->
//        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
//    {
//        unimplemented!();
//    }
}



pub struct Call
{
    pub sender   : AppMsgSink,
    pub receiver : AppMsgStream
}

trait DAppInit
{
    // Implies asking the user interface to manually pick a profile the app is used with
    fn initialize(&self, app: &ApplicationId, handle: &reactor::Handle)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >;
}


// NOTE A specific DApp is logged in to the Connect Service with given details, e.g. a selected profile.
//      A DApp might have several sessions, e.g. running in the name of multiple profiles.
pub trait DAppSession
{
    // Once initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> &ProfileId;

    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=::Error> >;

    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=::Error> >;

    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=::Error> >;

    // This includes initiating a pair request with the profile if not a relation yet
    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Call, Error=::Error> >;
}
