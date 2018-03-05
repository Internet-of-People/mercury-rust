extern crate futures;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

use std::rc::Rc;

use futures::{Future, Sink, Stream};
use futures::future;
use multiaddr::{Multiaddr};
use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

pub mod imp;



// TODO
pub enum ErrorToBeSpecified { TODO, }

pub enum SearchProfileError
{
    TODO // TODO
}

pub enum SearchAddressError
{
    TODO // TODO
}

pub enum CallContactError
{
    LookupFailed(SearchAddressError),
    RingFailed(String), // TODO
    Other(Box<std::error::Error>),
}



#[derive(Debug, Clone)]
pub struct PublicKey(Vec<u8>);
#[derive(Debug, Clone)]
pub struct ProfileId(multihash::Hash);
#[derive(Debug, Clone)]
pub struct Signature(Vec<u8>);
#[derive(Debug, Clone)]
pub struct ApplicationId(String);
#[derive(Debug, Clone)]
pub struct AppMessageFrame(String);

#[derive(Debug, Clone)]
pub struct PairingCertificate
{
    initiator_id:   ProfileId,
    acceptor_id:    ProfileId,
    initiator_sign: Signature,
    acceptor_sign:  Signature,
    // TODO is a nonce needed?
}

#[derive(Debug, Clone)]
pub struct HomeInvitation
{
    home_id: ProfileId,
    voucher: String,
    signature: Signature,
    // TODO is a nonce needed?
}



#[derive(Debug, Clone)]
pub struct PersonaTrait
{
    homes: Vec<ProfileId>,
}


#[derive(Debug, Clone)]
pub struct HomeTrait
{
    addrs: Vec<Multiaddr>,
}



// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(Debug, Clone)]
pub struct ApplicationTrait
{
    id: ApplicationId,
    // TODO
}


#[derive(Debug, Clone)]
pub struct Raw
{
    data: String,
}



#[derive(Debug, Clone)]
pub enum ProfileTrait
{
    Home(HomeTrait),
    Persona(PersonaTrait),
    Application(ApplicationTrait),
    Raw(String),
}


#[derive(Debug, Clone)]
pub struct Profile
{
    id:         ProfileId,
    pub_key:    PublicKey,
    traits:     Vec<ProfileTrait>,
}

impl Profile
{
    pub fn new(id: &ProfileId, pub_key: &PublicKey, traits: &[ProfileTrait]) -> Self
        { Self{ id: id.to_owned(), pub_key: pub_key.to_owned(), traits: traits.to_owned() } }
}



#[derive(Debug, Clone)]
pub struct Contact
{
    profile:    Profile,
    proof:      PairingCertificate,
}

impl Contact
{
    fn new(profile: &Profile, proof: &PairingCertificate) -> Self
        { Self { profile: profile.clone(), proof: proof.clone() } }
}



#[derive(Debug, Clone)]
pub struct OwnProfileData
{
    profile:    Profile,
// TODO
//    priv_metadata:   ???,
}

impl OwnProfileData
{
    pub fn new(profile: &Profile, /* TODO priv_metadata? */ ) -> Self
        { Self{ profile: profile.clone() } }
}



#[derive(Debug, Clone)]
pub struct SecretKey(Vec<u8>);

// NOTE implemented containing a SecretKey or something similar internally
pub trait Signer
{
    fn pub_key(&self) -> &PublicKey;
    // TODO Vec<u8> will be ideally the multicodec output from Mudlee's repo
    fn sign(&self, data: Vec<u8>) -> Signature;
}



pub struct OwnProfile
{
    profile: OwnProfileData,
    signer:  Rc<Signer>,
}




// Potentially a whole network of nodes with internal routing and sharding
pub trait ProfileRepo
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >;

    fn load(&self, id: ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // TODO notifications on profile updates should be possible
}



pub trait HomeConnector
{
    fn connect(&self, home: &HomeTrait) -> Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >;
    //fn resolve(&self, url: &str) -> Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >;
}



pub struct CallStream
{
    stream: Box< Stream<Item=Vec<u8>, Error=ErrorToBeSpecified> >,
    sink:   Box< Sink<SinkItem=Vec<u8>, SinkError=ErrorToBeSpecified> >,
}

pub struct Call
{
    caller:         ProfileId,
    init_payload:   Vec<u8>,
    // NOTE A missed call will contain Option::None
    stream:         Option<CallStream>,
}



// Interface to a single node
pub trait Home: ProfileRepo
{
    fn register(&self, prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // TODO consider if we should notify an open session about an updated profile
    fn update(&self, profile: OwnProfile) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
    fn unregister(&self, prof: OwnProfile, newhome: Option<Profile>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    fn claim(&self, profile: Profile, signer: Rc<Signer>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;


    // NOTE acceptor must have this server as its home
    fn pair_with(&self, initiator: &OwnProfile, acceptor: &Profile) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;

    fn call(&self, initiator: &OwnProfile, acceptor: &Contact,
            app: ApplicationId, init_payload: &[u8]) ->
        Box< Future<Item=CallStream, Error=ErrorToBeSpecified> >;


    fn login(&self, profile: &OwnProfile) ->
        Box< Future<Item=Box<Session>, Error=ErrorToBeSpecified> >;
}



pub trait Session
{
    // TODO add/remove app should be done in OwnProfile which should be delegated to Home?
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< Stream<Item=Call, Error=ErrorToBeSpecified> >;

    fn checkout_app(&self, app: &ApplicationId, calls: Stream<Item=Call, Error=ErrorToBeSpecified>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn banned_profiles(&self) ->
        Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >;

    fn ban(&self, profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn unban(&self, profile: &ProfileId) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



pub trait Client
{
    fn contacts(&self) -> Box< Stream<Item=Contact, Error=()> >;    // TODO error type
    fn profiles(&self) -> Box< Stream<Item=OwnProfile, Error=()> >; // TODO error type

    fn pair_with(&self, initiator: &OwnProfile, acceptor_profile_url: &str) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;

    fn call(&self, contact: &Contact, app: &ApplicationId) ->
        Box< Future<Item=Call, Error=CallContactError> >;

    fn login(&self, profile: &OwnProfile) -> Box< Future<Item=Box<Session>, Error=ErrorToBeSpecified> >;
}



pub struct ClientImp
{
    profile_repo:   Rc<ProfileRepo>,
    home_connector: Rc<HomeConnector>,
}


impl ClientImp
{

}


impl Client for ClientImp
{
    fn contacts(&self) -> Box< Stream<Item=Contact, Error=()> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        Box::new(recv)
    }


    fn profiles(&self) -> Box< Stream<Item=OwnProfile, Error=()> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        Box::new(recv)
    }


    fn pair_with(&self, initiator: &OwnProfile, acceptor_profile_url: &str) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )

//        self.profile_repo.resolve(acceptor_profile_url)
//            .and_then( |profile|
//            {
//                profile.traits.iter()
//                    .flat_map( |_trait|
//                        match _trait {
//                            &ProfileTrait::Persona(persona) => persona.homes,
//                            _ => Vec::new(),
//                        } )
//                    .collect()
//            } )
//            .and_then( |home_ids|
//            {
//                .map( |home_id| self.profile_repo.load(home_id) )
//                self.home_connector.connect( homes[0] )
//            } )
    }


    fn call(&self, contact: &Contact, app: &ApplicationId) ->
        Box< Future<Item=Call, Error=CallContactError> >
    {
        Box::new( future::err(CallContactError::RingFailed("TODO".to_string())) )

//        let result = contact.profile.find_addresses()
//            .map_err( |e| ConnectToContactError::LookupFailed(e) )
//            .and_then( |addrs|
//                {
//                    for addr in addrs
//                        {
//                        }
//                    future::err( ConnectToContactError::ConnectFailed(ConnectAddressError::TODO) )
//                } );
//
//        Box::new(result)
    }


    fn login(&self, profile: &OwnProfile) ->
        Box< Future<Item=Box<Session>, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
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
    fn test_connect_multiaddr()
    {
//        let mut setup = TestSetup::new();
//        let addr = "/ip4/127.0.0.1/tcp/12345".to_multiaddr().unwrap();
//        let connect_fut = connect(addr);
//        let result = setup.reactor.run(connect_fut);
//        // TODO assert!( result.TODO );
    }


    #[test]
    fn test_register_profile()
    {
//        let mut setup = TestSetup::new();
//        let ownprof = OwnProfile::new( &Profile::new( &"OwnProfileId".to_string() ) );
//        let addr = "/ip4/127.0.0.1/tcp/23456".to_multiaddr().unwrap();
//        let register_fut = ProfileIndex::new(&addr)
//            .and_then( move |host|
//                { host.register(&ownprof) } );
//        let result = setup.reactor.run(register_fut);
//        // TODO assert!( result.TODO );
    }


    #[test]
    fn test_claim_profile()
    {
        // TODO
    }

    #[test]
    fn test_pair_profiles()
    {
        // TODO
    }

    #[test]
    fn test_lookup_profiles()
    {
        // TODO
    }

    #[test]
    fn test_lookup_contacts()
    {
        // TODO
    }

    #[test]
    fn test_connect_profile_service()
    {
        // TODO
    }

    #[test]
    fn test_appservice_connect()
    {
        // TODO
    }

    #[test]
    fn test_appservice_listen()
    {
        // TODO
    }
}
