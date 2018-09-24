#[macro_use]
extern crate failure;
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

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::error::*;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppPermission(Vec<u8>);



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



pub trait ConnectService
{
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >;

    // TODO The Settings app is not really a dApp but a privilegized system app, might use different authorization
    // TODO assigning a UI is more like an initialization process than an Rpc endpoint, reconsider the ui argument
    fn admin_endpoint(&self, // ui: Rc<UserInterface>,
                      authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<AdminEndpoint>, Error=Error> >;
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



pub trait AdminEndpoint
{
    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;
    // fn claim(&self, profile_path: TODO_profileId_or_Bip32PAth?) -> Box< Future<Item=Rc<OwnProfile>, Error=Error> >;
    fn create_profile(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;
    fn update_profile(&self, profile: &OwnProfile) -> Box< Future<Item=(), Error=Error> >;
    fn remove_profile(&self, profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;

    fn homes(&self, profile: &ProfileId) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn join_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
    fn leave_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;

    fn relations(&self, profile: &ProfileId) -> Box< Future<Item=Vec<Relation>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn initiate_relation(&self, my_profile: &ProfileId, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;
    fn accept_relation(&self, half_proof: &RelationHalfProof) -> Box< Future<Item=(), Error=Error> >;
    fn revoke_relation(&self, profile: &ProfileId, relation: &RelationProof) -> Box< Future<Item=(), Error=Error> >;
}
