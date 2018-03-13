extern crate capnp;
extern crate futures;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

use std::rc::Rc;

use futures::{Future, IntoFuture, Sink, Stream};
use futures::future;
use multiaddr::{Multiaddr};
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
pub struct PublicKey(pub Vec<u8>);
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ProfileId(pub Vec<u8>); // NOTE multihash::Multihash::encode() output
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Signature(pub Vec<u8>);
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ApplicationId(pub String);
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AppMessageFrame(pub Vec<u8>);


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PairingCertificate
{
    pub initiator_id:   ProfileId,
    pub acceptor_id:    ProfileId,
    pub initiator_sign: Signature,
    pub acceptor_sign:  Signature,
    // TODO is a nonce needed?
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeInvitation
{
    pub home_id: ProfileId,
    pub voucher: String,
    pub signature: Signature,
    // TODO is a nonce needed?
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PersonaFacet
{
    pub homes: Vec<ProfileId>,
    // TODO and probably a lot more data
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeFacet
{
    pub addrs: Vec<Multiaddr>,
    // TODO and probably a lot more data
}


// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ApplicationFacet
{
    pub id: ApplicationId,
    // TODO and probably a lot more data
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
    Raw(String),
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Profile
{
    pub id:         ProfileId,
    pub pub_key:    PublicKey,
    pub facets:     Vec<ProfileFacet>, // TODO consider using dictionary instead of vector
}

impl Profile
{
    pub fn new(id: &ProfileId, pub_key: &PublicKey, facets: &[ProfileFacet]) -> Self
        { Self{ id: id.to_owned(), pub_key: pub_key.to_owned(), facets: facets.to_owned() } }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Contact
{
    pub profile:    Profile,
    pub proof:      PairingCertificate,
}

impl Contact
{
    fn new(profile: &Profile, proof: &PairingCertificate) -> Self
        { Self { profile: profile.clone(), proof: proof.clone() } }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct OwnProfileData
{
    pub profile:    Profile,
    pub priv_data:  Vec<u8>, // TODO maybe multicodec output?
}

impl OwnProfileData
{
    pub fn new(profile: &Profile, private_data: &[u8]) -> Self
        { Self{ profile: profile.clone(), priv_data: private_data.to_owned() } }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct SecretKey(Vec<u8>);

// NOTE implemented containing a SecretKey or something similar internally
pub trait Signer
{
    fn pub_key(&self) -> &PublicKey;
    // TODO the data Vec<u8> to be signed ideally will be the output from Mudlee's multicodec lib
    fn sign(&self, data: Vec<u8>) -> Signature;
}



#[derive(Clone)]
pub struct OwnProfile
{
    pub data:   OwnProfileData,
    pub signer: Rc<Signer>,
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



// Interface to a single node
pub trait Home: ProfileRepo
{
    fn register(&self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // TODO consider if we should notify an open session about an updated profile
    fn update(&self, own_prof: OwnProfile) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    fn unregister(&self, own_prof: OwnProfile, newhome: Option<Profile>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    fn claim(&self, profile: Profile, signer: Rc<Signer>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;


    // NOTE acceptor must have this server as its home
    fn pair_with(&self, initiator: OwnProfile, acceptor: Profile) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;

    fn call(&self, caller: OwnProfile, callee: Contact,
            app: ApplicationId, init_payload: &[u8]) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >;


    fn login(&self, own_prof: OwnProfile) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >;
}



// TODO maybe use names like ProfileEvent and ProfileSession?
pub enum HomeEvent
{
    // TODO what other events are needed?
    PairingRequest,
    PairingResponse,
// ProfileUpdated // from a different client instance/session
}


pub trait HomeSession
{
    fn events(&self) -> Rc< Stream<Item=HomeEvent, Error=ErrorToBeSpecified> >;

    // TODO return not a Stream, but an AppSession struct containing a stream
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< Stream<Item=Call, Error=ErrorToBeSpecified> >;

    // TODO this is probably not needed, we'll just drop the checkin_app result object instead
//    fn checkout_app(&self, app: &ApplicationId, calls: Stream<Item=Call, Error=ErrorToBeSpecified>) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn banned_profiles(&self) ->
        Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >;

    fn ban(&self, profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn unban(&self, profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



#[cfg(test)]
mod tests
{
    use super::*;
    use multiaddr::ToMultiaddr;


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
