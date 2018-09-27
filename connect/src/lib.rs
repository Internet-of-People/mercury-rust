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



pub mod profile;
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


pub struct DAppCall
{
    pub sender   : AppMsgSink,
    pub receiver : AppMsgStream
}


pub enum DAppEvent
{
    PairingResponse(RelationProof),
    Call(Box<IncomingCall>),
}



pub trait DAppEndpoint
{
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >;
}



// NOTE A specific DApp is logged in to the Connect Service with given details, e.g. a selected profile.
//      A DApp might have several sessions, e.g. running in the name of multiple profiles.
pub trait DAppSession
{
    // After the session was initialized, the profile is selected and can be queried any time
    fn selected_profile(&self) -> &ProfileId;

    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=::Error> >;

    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=::Error> >;

    fn checkin(&self)
        -> Box< Future<Item=Box<Stream<Item=Result<DAppEvent,String>, Error=()>>, Error=::Error> >;

    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=DAppCall, Error=::Error> >;
}



pub fn find_relation_proof<'a>(relations: &'a [Relation], my_id: ProfileId, peer_id: ProfileId,
    relation_type: Option<&str>) -> Option<RelationProof>
{
    let mut peers_filtered = relations.iter()
        .map( |relation| relation.proof.clone() )
        .filter( |proof| {
            proof.peer_id(&my_id)
                .map( |p_id| *p_id == peer_id )
                .unwrap_or(false)
        } );

    match relation_type {
        None      => peers_filtered.next(),
        Some(rel) => peers_filtered
            .filter( |proof| proof.relation_type == rel ).next(),
    }
}
