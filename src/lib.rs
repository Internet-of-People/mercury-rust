extern crate futures;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

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

pub enum ConnectAddressError
{
    TODO // TODO
}

pub enum ConnectToContactError
{
    LookupFailed(SearchAddressError),
    ConnectFailed(ConnectAddressError),
    Other(Box<std::error::Error>),
}



#[derive(Debug, Clone)]
pub struct PublicKey(Vec<u8>);
#[derive(Debug, Clone)]
pub struct ProfileId(multihash::Hash);
#[derive(Debug, Clone)]
pub struct MetadataSchema(String);
#[derive(Debug, Clone)]
pub struct ApplicationId(String);

#[derive(Debug, Clone)]
pub struct PairingCertificate
{
    initiator_id:   ProfileId,
    acceptor_id:    ProfileId,
    // TODO
    // signatures: ???
}

#[derive(Debug, Clone)]
pub struct HomeInvitation
{
    // TODO ???
    // voucher: code,
    // signature: ???,
}



#[derive(Debug, Clone)]
pub struct PersonaSchema
{
    homes: Vec<ProfileId>,
}


#[derive(Debug, Clone)]
pub struct HomeSchema
{

    addrs: Vec<Multiaddr>,
}


// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(Debug, Clone)]
pub struct ApplicationSchema
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
pub enum Metadata
{
    Home(HomeSchema),
    Persona(PersonaSchema),
    Application(ApplicationSchema),
    Raw(String),
}


#[derive(Debug, Clone)]
pub struct Profile
{
    id:         ProfileId,
    pub_key:    PublicKey,
    metadata:   Vec<Metadata>,
}

impl Profile
{
    pub fn new(id: &ProfileId, pub_key: &PublicKey, metadata: &[Metadata]) -> Self
        { Self{ id: id.to_owned(), pub_key: pub_key.to_owned(), metadata: metadata.to_owned() } }
}



#[derive(Debug, Clone)]
pub struct Contact
{
    profile:    Profile,
    proof:      PairingCertificate,
}

impl Contact
{
    // TODO
    // fn new(...) -> Self
}



#[derive(Debug, Clone)]
pub struct OwnProfile
{
    profile:    Profile,
// TODO
//    priv_metadata:   ???,
}

impl OwnProfile
{
    pub fn new(profile: &Profile, /* TODO apps, priv_metadata? */ ) -> Self
        { Self{ profile: profile.clone() } }
}



#[derive(Debug, Clone)]
pub struct SecretKey(Vec<u8>);

// NOTE implemented containing a SecretKey or something similar internally
pub trait Signer
{
    fn pub_key(&self) -> &PublicKey;
    fn sign(&self, data: Vec<u8>) -> Vec<u8>;
}



// Potentially a whole network of nodes with internal routing and sharding
// NOTE to construct an object of this type, a Signer instance will be needed
//      to prove our identity during each method call
pub trait ProfileStorage
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >;

    fn load(&self, id: ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // TODO notifications on profile updates should be possible
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
// NOTE to construct an object of this type, a Signer instance will be needed
//      to prove our identity during each method call
pub trait ForeignHome : ProfileStorage
{
    fn register(&self, prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
    fn unregister(&self, prof: OwnProfile, newhome: Option<Profile>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    fn claim(&self, profile: Profile /* TODO what other params to prove for ownership? */ ) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // NOTE acceptor must have this server as its home
    fn pair_with(&self, initiator: &OwnProfile, acceptor: &Profile) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;

    fn call(&self, initiator: &OwnProfile, acceptor: &Contact,
            app: ApplicationId, init_payload: &[u8]) ->
        Box< Future<Item=CallStream, Error=ErrorToBeSpecified> >;
}



pub trait OwnHome
{
    fn login(&self, profile: &OwnProfile, signer: &Signer) ->
        Box< Future<Item=Box<OwnSession>, Error=ErrorToBeSpecified> >;

    // TODO consider if we should notify an open session about an updated profile
    fn update(&self, profile: OwnProfile, signer: &Signer) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;
}



// NOTE to construct an object of this type, a Signer instance will be needed
//      to prove our identity during each method call
pub trait OwnSession
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



// TODO this is not properly thought out yet
//pub trait Client
//{
////    // NOTE loads up profile from ProfileStorage and connects appropriate ForeignHome for pairing
////    fn pair_with(&self, initiator: &OwnProfile, acceptor: &Profile) ->
////        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;
//
//    fn connect_to<A: 'static + AsyncRead + AsyncWrite>(&self, contact: &Contact, app: &ApplicationId) ->
//        Box< Future<Item=A, Error=ConnectToContactError>>
//    {
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
//    }
//
//    fn list( /* TODO what filter criteria should we have here? */ ) ->
//        Box< Future<Item=Vec<Profile>, Error=SearchProfileError> >
//    {
//        // TODO explore ProfileHosts and query profiles on each of them
//        Box::new( future::err(SearchProfileError::TODO) )
//    }
//
//    // TODO should we use Stream<Multiaddr> here instead of Vec<>?
//    fn find_addresses(&self) ->
//        Box< Future<Item=Vec<Multiaddr>, Error=SearchAddressError> >
//    {
//        // TODO explore profile Home and query known addresses for profile
//        Box::new( future::err(SearchAddressError::TODO) )
//    }
//
//    fn call(profile: &ProfileId, app: &ApplicationId) -> TODO;
//}



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
        let mut setup = TestSetup::new();
        let addr = "/ip4/127.0.0.1/tcp/12345".to_multiaddr().unwrap();
        let connect_fut = connect(addr);
        let result = setup.reactor.run(connect_fut);
        // TODO assert!( result.TODO );
    }


    #[test]
    fn test_register_profile()
    {
        let mut setup = TestSetup::new();
        let ownprof = OwnProfile::new( &Profile::new( &"OwnProfileId".to_string() ) );
        let addr = "/ip4/127.0.0.1/tcp/23456".to_multiaddr().unwrap();
        let register_fut = ProfileIndex::new(&addr)
            .and_then( move |host|
                { host.register(&ownprof) } );
        let result = setup.reactor.run(register_fut);
        // TODO assert!( result.TODO );
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
