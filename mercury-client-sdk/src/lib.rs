extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_common;
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

use mercury_common::*;

pub mod net;



pub trait HomeConnector
{
    // NOTE home_profile must have a HomeFacet with at least an address filled in
    fn connect(&self, home_profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >;
}



pub trait Client
{
// TODO consider if using streams here is a good idea considering implementation complexity
//    fn contacts(&self) -> Box< Stream<Item=Contact, Error=()> >;    // TODO error type
//    fn profiles(&self) -> Box< Stream<Item=OwnProfile, Error=()> >; // TODO error type

    fn contacts(&self) -> Box< Future<Item=Vec<Contact>, Error=ErrorToBeSpecified> >;
    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >;


    fn register(&self, home: ProfileId, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

//    fn update(&self, home: ProfileId, own_prof: OwnProfile) ->
//        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;
//
//    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
//    fn unregister(&self, home: ProfileId, own_prof: OwnProfile, newhome: Option<Profile>) ->
//        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    fn claim(&self, home: ProfileId, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;


    fn pair_with(&self, initiator: Profile, acceptor_profile_url: &str) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >;

    fn call(&self, caller: Profile, callee: Contact,
            app: ApplicationId, init_payload: Vec<u8>) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >;

    fn login(&self, own_prof: Profile) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >;

    // TODO what else is needed here?
}



#[derive(Clone)]
pub struct ClientImp
{
    profile_repo:   Rc<ProfileRepo>,
    home_connector: Rc<HomeConnector>,
}


impl ClientImp
{
    fn connect_home(&self, home_profile_id: &ProfileId) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let home_connector_clone = self.home_connector.clone();
        let home_conn_fut = self.profile_repo.load(home_profile_id)
            .and_then( move |home_profile|
                home_connector_clone.connect(&home_profile) );
        Box::new(home_conn_fut)
    }


    fn any_home_of(&self, profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let profile_repo_clone = self.profile_repo.clone();
        let home_connector_clone = self.home_connector.clone();
        ClientImp::any_home_of2(profile, profile_repo_clone, home_connector_clone)
    }


    fn any_home_of2(profile: &Profile, prof_repo: Rc<ProfileRepo>, connector: Rc<HomeConnector>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let home_conn_futs = profile.facets.iter()
            .flat_map( |facet|
            {
                match facet
                {
                    // TODO consider how to get homes/addresses for apps and smartfridges
                    &ProfileFacet::Persona(ref persona) => persona.homes.clone(),
                    _ => Vec::new(),
                }
            } )
            .map( move |home_prof_id|
            {
                // Load profiles from home ids
                let home_connector_clone = connector.clone();
                prof_repo.load(&home_prof_id)
                    .and_then( move |home_prof|
                    {
                        // Connect to loaded homeprofile (Home of the user to pair with)
                        home_connector_clone.connect(&home_prof)
                    } )
            } );

        // Pick first successful home connection
        let home_conn_fut = future::select_ok( home_conn_futs )
            .map( |(home_conn, _pending_conn_futs)| home_conn );
        Box::new(home_conn_fut)
    }
}


impl Client for ClientImp
{
//    fn contacts(&self) -> Box< Stream<Item=Contact, Error=()> >
//    {
//        // TODO
//        let (send, recv) = futures::sync::mpsc::channel(0);
//        Box::new(recv)
//    }
//
//
//    fn profiles(&self) -> Box< Stream<Item=OwnProfile, Error=()> >
//    {
//        // TODO
//        let (send, recv) = futures::sync::mpsc::channel(0);
//        Box::new(recv)
//    }


    fn contacts(&self) -> Box< Future<Item=Vec<Contact>, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }


    fn register(&self, home_id: ProfileId, own_prof: OwnProfile,
                invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        let reg_fut = self.connect_home(&home_id)
            .and_then( move |home| home.register(own_prof, invite) );
        Box::new(reg_fut)
    }

//    fn update(&self, home_id: ProfileId, own_prof: OwnProfile) ->
//        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
//    {
//        let upd_fut = self.connect_home(&home_id)
//            .and_then( move |home| home.update(own_prof) );
//        Box::new(upd_fut)
//    }
//
//    fn unregister(&self, home_id: ProfileId, own_prof: OwnProfile, newhome: Option<Profile>) ->
//        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
//    {
//        let unreg_fut = self.connect_home(&home_id)
//            .and_then( move |home| home.unregister(own_prof, newhome) );
//        Box::new(unreg_fut)
//    }

    fn claim(&self, home_id: ProfileId, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        let claim_fut = self.connect_home(&home_id)
            .and_then( move |home| home.claim(profile) );
        Box::new(claim_fut)
    }


    fn pair_with(&self, initiator: Profile, acceptor_profile_url: &str) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >
    {
        let profile_repo_clone = self.profile_repo.clone();
        let home_connector_clone = self.home_connector.clone();

        let pair_fut = self.profile_repo
            .resolve(acceptor_profile_url)
            .and_then( |profile|
            {
                ClientImp::any_home_of2(&profile, profile_repo_clone, home_connector_clone)
                    .and_then( move |home|
                        home.pair_with(initiator, profile) )
            } );

        Box::new(pair_fut)
    }


    fn call(&self, caller: Profile, callee: Contact,
            app: ApplicationId, init_payload: Vec<u8>) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >
    {
        let pair_fut = self.any_home_of(&callee.profile)
            .and_then( move |home|
                home.call( caller, callee, app, init_payload.as_slice() ) ) ;
        Box::new(pair_fut)
    }


    // TODO this should try connecting to ALL of our homes
    fn login(&self, own_prof: Profile) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        let pair_fut = self.any_home_of(&own_prof)
            .and_then( move |home|
                home.login(own_prof) ) ;

        Box::new(pair_fut)
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


    struct DummySigner
    {
        prof_id: ProfileId,
        pub_key: PublicKey,
    }

    impl Signer for DummySigner
    {
        fn prof_id(&self) -> &ProfileId { &self.prof_id }
        fn pub_key(&self) -> &PublicKey { &self.pub_key }
        fn sign(&self, data: Vec<u8>) -> Signature { Signature( Vec::new() ) }
    }

    #[test]
    fn temporary_test_capnproto()
    {
        use std::net::SocketAddr;
        use std::net::ToSocketAddrs;
        use super::net::*;

        let mut setup = TestSetup::new();

        let prof_id = &ProfileId( "joooozsi".as_bytes().to_owned() );
        let profile = Profile::new( prof_id,
            &PublicKey( "publickey".as_bytes().to_owned() ), &[] );
        let signer = Rc::new( DummySigner{ prof_id: prof_id.clone(), pub_key: PublicKey(Vec::new()) } );
        let own_profile = OwnProfile::new(
            OwnProfileData::new( &profile, &[] ),
            signer);

        let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
        let handle = setup.reactor.handle();
        let test_fut = TcpStream::connect( &addr, &setup.reactor.handle() )
            .map_err( |_e| ErrorToBeSpecified::TODO )
            .and_then( move |tcp_stream|
            {
                let home = HomeClientCapnProto::new(tcp_stream, handle);
                home.login(own_profile)
            } )
            .and_then( |session| session.ping("hahoooo") );

        let pong = setup.reactor.run(test_fut);
        println!("Response: {:?}", pong);
//        // TODO assert!( result.TODO );
    }
}