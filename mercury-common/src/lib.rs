#![allow(unused)]
extern crate capnp;
extern crate futures;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

use std::collections::hash_map::HashMap;
use std::rc::Rc;

use futures::{Future, IntoFuture, Sink, Stream};
use futures::future;
use multiaddr::Multiaddr;
use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};



pub mod mercury_capnp {
    include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );
}


// TODO
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ErrorToBeSpecified { TODO, }



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ProfileId(pub Vec<u8>); // NOTE multihash::Multihash::encode() output

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PublicKey(pub Vec<u8>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Signature(pub Vec<u8>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Bip32Path(String);



pub trait Seed
{
    // TODO do we need a password to unlock the private key?
    fn unlock(bip32_path: &Bip32Path) -> Rc<Signer>;
}


// NOTE implemented containing a SecretKey or something similar internally
pub trait Signer
{
    fn prof_id(&self) -> &ProfileId; // TODO is this really needed here?
    fn pub_key(&self) -> &PublicKey;
    // NOTE the data Vec<u8> to be signed ideally will be the output from Mudlee's multicodec lib
    fn sign(&self, data: Vec<u8>) -> Signature;
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PersonaFacet
{
    pub homes:  Vec<Relation>, // NOTE with proof relation_type "home"
    pub data:   Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeFacet
{
    pub addrs:  Vec<Multiaddr>,
    pub data:   Vec<u8>,
}

// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ApplicationFacet
{
    pub id:     ApplicationId,
    pub data:   Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RawFacet
{
    pub data: Vec<u8>, // TODO or maybe multicodec output?
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ProfileFacet
{
    Home(HomeFacet),
    Persona(PersonaFacet),
    Application(ApplicationFacet),
    Unknown(RawFacet),
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Profile
{
    pub id:         ProfileId,
    pub pub_key:    PublicKey,
    pub facets:     Vec<ProfileFacet>, // TODO consider redesigning facet Rust types/storage
    // TODO consider having a signature of the profile data here
}


impl Profile
{
    pub fn new(id: &ProfileId, pub_key: &PublicKey, facets: &[ProfileFacet]) -> Self
        { Self{ id: id.to_owned(), pub_key: pub_key.to_owned(), facets: facets.to_owned() } }
}



pub trait PeerContext
{
    fn my_signer(&self) -> &Signer;
    fn peer_pubkey(&self) -> Option<PublicKey>;
    fn peer(&self) -> Option<Profile>;
}



// Potentially a whole network of nodes with internal routing and sharding
pub trait ProfileRepo
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >;

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // TODO notifications on profile updates should be possible
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct OwnProfile
{
    pub profile:    Profile,
    pub priv_data:  Vec<u8>, // TODO maybe multicodec output?
}

impl OwnProfile
{
    pub fn new(profile: &Profile, private_data: &[u8]) -> Self
    { Self{ profile: profile.clone(), priv_data: private_data.to_owned() } }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RelationHalfProof
{
    pub relation_type:  String,
    pub my_id:          ProfileId,
    pub my_sign:        Signature,
    pub peer_id:        ProfileId,
    // TODO is a nonce needed?
}


// TODO maybe halfproof should be inlined (with macro?)
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RelationProof
{
    pub half_proof: RelationHalfProof,
    pub peer_sign:  Signature,
    // TODO is a nonce needed?
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Relation
{
    pub profile:    Profile,
    pub proof:      RelationProof,
}

impl Relation
{
    fn new(profile: &Profile, proof: &RelationProof) -> Self
    { Self { profile: profile.clone(), proof: proof.clone() } }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeInvitation
{
    pub home_id:    ProfileId,
    pub voucher:    String,
    pub signature:  Signature,
    // TODO is a nonce needed?
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ApplicationId(pub String);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AppMessageFrame(pub Vec<u8>);


pub struct CallMessages
{
    pub incoming: Box< Stream<Item=AppMessageFrame, Error=ErrorToBeSpecified> >,
    pub outgoing: Box< Sink<SinkItem=AppMessageFrame, SinkError=ErrorToBeSpecified> >,
}

pub struct Call
{
    pub caller:         ProfileId,
    pub init_payload:   AppMessageFrame,
    // NOTE A missed call will contain Option::None
    pub messages:       Option<CallMessages>,
}



// Interface to a single home server.
// NOTE authentication is already done when the connection is built,
//      authenticated profile info is available from the connection context
pub trait Home: PeerContext + ProfileRepo
{
    // NOTE profile id needed because of supporting multihash, the id cannot be guessed otherwise
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >;

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >;


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >;

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}



pub enum ProfileEvent
{
    Unknown(Vec<u8>), // forward compatibility for protocol extension
    PairingRequest,
    PairingResponse,
// TODO are these events needed? What others?
    HomeBroadcast,
    HomeHostingExpiry,
    ProfileUpdated, // from a different client instance/session
}


pub trait HomeSession
{
    fn update(&self, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn events(&self) -> Rc< Stream<Item=ProfileEvent, Error=ErrorToBeSpecified> >;

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< Stream<Item=Call, Error=ErrorToBeSpecified> >;

    // TODO remove this after testing
    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >;


// TODO ban features are delayed to a later milestone
//    fn banned_profiles(&self) ->
//        Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >;
//
//    fn ban(&self, profile: &ProfileId) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//
//    fn unban(&self, profile: &ProfileId) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



#[cfg(test)]
mod tests
{
    use super::*;


    struct TestSetup
    {
        reactor: reactor::Core,
    }

    impl TestSetup
    {
        fn new() -> Self
        {
            Self{ reactor: reactor::Core::new().unwrap() }
        }
    }

    #[test]
    fn test_something()
    {
//        // TODO assert!( result.TODO );
    }
}
