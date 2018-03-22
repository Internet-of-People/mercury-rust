extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_common;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

use std::rc::Rc;

use futures::{Future, Stream}; // IntoFuture, Sink
use futures::future;

use mercury_common::*;

pub mod net;



pub trait HomeConnector
{
    // NOTE home_profile must have a HomeFacet with at least an address filled in
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >;
}



pub trait ProfileConnector
{
    fn next() -> Bip32Path;
    fn connect(bip32_path: &Bip32Path) -> Rc<ProfileGateway>;
}



pub struct ClientHomeContext
{
    signer:         Rc<Signer>,
    home_profile:   Profile,
}

impl ClientHomeContext
{
    pub fn new(signer: Rc<Signer>, home_profile: &Profile) -> Self
        { Self{ signer: signer, home_profile: home_profile.clone() } }
}

impl HomeContext for ClientHomeContext
{
    fn signer(&self) -> &Signer { &*self.signer }
    fn peer(&self) -> Option<Profile> { Some( self.home_profile.clone() ) }
    fn peer_pubkey(&self) -> Option<PublicKey> { Some( self.home_profile.pub_key.clone() ) }
}



pub trait ProfileGateway
{
// TODO consider if using streams here is a good idea considering implementation complexity
//    fn relations(&self) -> Box< Stream<Item=Contact, Error=()> >;   // TODO error type
//    fn profiles(&self) -> Box< Stream<Item=OwnProfile, Error=()> >; // TODO error type

// NOTE this interface currently works with a single Profile, so this is not here
//    fn profiles(&self) ->
//        Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >;
    fn relations(&self, profile: &ProfileId) ->
        Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >;


    // TODO do we really want only profileId of homes here would a Profile also comfortable?
    fn claim(&self, home: ProfileId, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    fn register(&self, home: ProfileId, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >;

    fn update(&self, home: ProfileId, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
    fn unregister(&self, home: ProfileId, own_prof: ProfileId, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn login(&self) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >;


    fn pair_request(&self, relation_type: &str, with_profile_url: &str) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >;

    // TODO what else is needed here?
}



#[derive(Clone)]
pub struct ProfileGatewayImpl
{
    signer:         Rc<Signer>,
    profile_repo:   Rc<ProfileRepo>,
    home_connector: Rc<HomeConnector>,
}


impl ProfileGatewayImpl
{
    fn connect_home(&self, home_profile_id: &ProfileId) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let home_connector_clone = self.home_connector.clone();
        let signer_clone = self.signer.clone();
        let home_conn_fut = self.profile_repo.load(home_profile_id)
            .and_then( move |home_profile|
                home_connector_clone.connect(&home_profile, signer_clone) );
        Box::new(home_conn_fut)
    }


    fn any_home_of(&self, profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let profile_repo_clone = self.profile_repo.clone();
        let home_connector_clone = self.home_connector.clone();
        let signer_clone = self.signer.clone();
        ProfileGatewayImpl::any_home_of2(profile, profile_repo_clone, home_connector_clone, signer_clone)
    }


    fn any_home_of2(profile: &Profile, prof_repo: Rc<ProfileRepo>,
                    connector: Rc<HomeConnector>, signer: Rc<Signer>) ->
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
            .map( move |home_relation|
            {
                // Load profiles from home ids
                connector.connect( &home_relation.profile, signer.clone() )
            } );

        // Pick first successful home connection
        let home_conn_fut = future::select_ok( home_conn_futs )
            .map( |(home_conn, _pending_conn_futs)| home_conn );
        Box::new(home_conn_fut)
    }


    fn new_half_proof(relation_type: &str, with_prof: &ProfileId, signer: Rc<Signer>) ->
        RelationHalfProof
    {
        // TODO implement binary serialization for signing
        RelationHalfProof{ relation_type: relation_type.to_owned(),
            my_id: signer.prof_id().to_owned(), peer_id: with_prof.to_owned(),
            my_sign: signer.sign( "TODO implement halfproof serialization".as_bytes().to_owned() ) }
    }
}


impl ProfileGateway for ProfileGatewayImpl
{
//    fn relations(&self) -> Box< Stream<Item=Contact, Error=()> >
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


//    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=ErrorToBeSpecified> >
//    {
//        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
//    }

    fn relations(&self, profile: &ProfileId) ->
        Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }


    fn claim(&self, home_id: ProfileId, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        let claim_fut = self.connect_home(&home_id)
            .and_then( move |home| home.claim(profile) );
        Box::new(claim_fut)
    }

    fn register(&self, home_id: ProfileId, own_prof: OwnProfile,
                invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >
    {
        let own_prof_clone = own_prof.clone();
        let reg_fut = self.connect_home(&home_id)
            .map_err( move |e| (own_prof_clone, e) )
            .and_then( move |home| home.register(own_prof, invite) );
        Box::new(reg_fut)
    }

    fn update(&self, home_id: ProfileId, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let own_profile_clone = own_prof.clone();
        let own_profile_id_clone = own_prof.profile.id.clone();
        let upd_fut = self.connect_home(&home_id)
            .and_then( move |home| home.login(own_profile_id_clone) )
            .and_then( move |session| session.update(&own_profile_clone) );
        Box::new(upd_fut)
    }

    fn unregister(&self, home_id: ProfileId, own_prof: ProfileId, newhome_id: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let unreg_fut = self.connect_home(&home_id)
            .and_then( move |home| home.login(own_prof) )
            .and_then( move |session| session.unregister(newhome_id) );
        Box::new(unreg_fut)
    }

    // TODO this should try connecting to ALL of our homes
    fn login(&self) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        let profile_repo_clone = self.profile_repo.clone();
        let home_conn_clone = self.home_connector.clone();
        let signer_clone = self.signer.clone();
        let prof_id = self.signer.prof_id().clone();
        let log_fut = self.profile_repo.load( &self.signer.prof_id() )
            .and_then( move |profile| ProfileGatewayImpl::any_home_of2(
                &profile, profile_repo_clone, home_conn_clone, signer_clone) )
            .and_then( move |home| home.login(prof_id) ) ;

        Box::new(log_fut)
    }


    fn pair_request(&self, relation_type: &str, with_profile_url: &str) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let profile_repo_clone = self.profile_repo.clone();
        let home_connector_clone = self.home_connector.clone();
        let signer_clone = self.signer.clone();
        let rel_type_clone = relation_type.to_owned();

        let pair_fut = self.profile_repo
            .resolve(with_profile_url)
            .and_then( move |profile|
            {
                let half_proof = ProfileGatewayImpl::new_half_proof(rel_type_clone.as_str(), &profile.id, signer_clone.clone() );
                ProfileGatewayImpl::any_home_of2(&profile, profile_repo_clone, home_connector_clone, signer_clone)
                    .and_then( move |home| home.pair_request(half_proof) )
            } );

        Box::new(pair_fut)
    }


    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let pair_fut = self.any_home_of(&rel.profile)
            .and_then( |home| home.pair_response(rel) );
        Box::new(pair_fut)
    }


    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >
    {
        let call_fut = self.any_home_of(&rel.profile)
            .and_then( move |home|
                home.call(rel, app, init_payload) ) ;
        Box::new(call_fut)
    }
}



#[cfg(test)]
mod tests
{
    use super::*;
    use tokio_core::net::TcpStream;
    use tokio_core::reactor;


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

        let prof_id = ProfileId( "joooozsi".as_bytes().to_owned() );
        let home_id = ProfileId( "HomeSweetHome".as_bytes().to_owned() );
        let signer = Rc::new( DummySigner{ prof_id: prof_id.clone(), pub_key: PublicKey(Vec::new()) } );
        let home_facet = HomeFacet{ addrs: Vec::new() };
        let home_prof = Profile::new( &home_id,
            &PublicKey( "HomePubKey".as_bytes().to_owned() ),
            &[ ProfileFacet::Home(home_facet) ] );
        let home_ctx = Box::new( ClientHomeContext::new(signer, &home_prof) );

//        let profile = Profile::new( prof_id,
//            &PublicKey( "publickey".as_bytes().to_owned() ), &[] );
//        let own_profile = OwnProfile::new( &profile, &[] );

        let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
        let handle = setup.reactor.handle();
        let test_fut = TcpStream::connect( &addr, &setup.reactor.handle() )
            .map_err( |_e| ErrorToBeSpecified::TODO )
            .and_then( move |tcp_stream|
            {
                let home = HomeClientCapnProto::new(tcp_stream, home_ctx, handle);
                home.login(prof_id)
            } )
            .and_then( |session| session.ping("hahoooo") );

        let pong = setup.reactor.run(test_fut);
        println!("Response: {:?}", pong);
//        // TODO assert!( result.TODO );
    }
}
