extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
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

use std::rc::Rc;

use futures::{future, Future};

use mercury_home_protocol::*;



pub mod net;
pub use net::SimpleTcpHomeConnector;

pub mod protocol_capnp;
pub mod sdk;

pub mod simple_profile_repo;
pub use simple_profile_repo::SimpleProfileRepo;



pub trait HomeConnector
{
    /// Initiate a permanent connection to the home server defined by `home_profile`, or return an
    /// existing, live `Home` immediately.
    /// `home_profile` must have a HomeFacet with at least an address filled in.
    /// `signer` belongs to me.
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >;
}



// TODO maybe this should be transformed to store a relationproof with an operation like
//      fn profile(&self) -> Box<Future<Item=Profile,Error=SomeError>>
//      cache profile after fetched in something like an Option<RefCell<Profile>>
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Relation
{
    pub proof:      RelationProof,
// TODO consider transforming Profile to Option<RefCell<Profile>> with an operation like
//      fn peer(&self) -> Box<Future<Item=Profile,Error=SomeError>>
//      which could return a cache profile value immediately or load it if not present yet
    pub peer:       Profile,
}

impl Relation
{
    pub fn new(peer: &Profile, proof: &RelationProof) -> Self
        { Self { peer: peer.clone(), proof: proof.clone() } }

    pub fn call(&self, init_payload: AppMessageFrame,
                to_caller: Option<AppMsgSink>) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
    {
        unimplemented!();
    }
}



pub trait ProfileGateway
{
    fn signer(&self) -> &Signer;
    fn relations(&self) -> Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >;

    fn claim(&self, home: ProfileId, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    /// `invite` is needed only if the home has a restrictive registration policy.
    fn register(&self, home: ProfileId, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >;

    fn update(&self, home: ProfileId, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeSchema different than this home
    fn unregister(&self, home: ProfileId, own_prof: ProfileId, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn pair_request(&self, relation_type: &str, with_profile_url: &str) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame,
            to_caller: Option<AppMsgSink>) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >;


    fn login(&self) ->
        Box< Future<Item=Rc<HomeSession>, Error=ErrorToBeSpecified> >;
}



#[derive(Clone)]
pub struct ProfileGatewayImpl
{
    pub signer:         Rc<Signer>,
    //local profile repository?
    pub profile_repo:   Rc<ProfileRepo>,
    pub home_connector: Rc<HomeConnector>,
}


impl ProfileGatewayImpl
{
    pub fn new(
        signer:         Rc<Signer>,
        profile_repo:   Rc<ProfileRepo>,
        home_connector: Rc<HomeConnector>,
    ) -> Self
    {
        ProfileGatewayImpl{
            signer:         signer,
            profile_repo:   profile_repo,
            home_connector: home_connector,
        }

    }

    pub fn connect_home(&self, home_profile_id: &ProfileId)
        -> Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        Self::connect_home2( home_profile_id, self.profile_repo.clone(),
                             self.home_connector.clone(), self.signer.clone() )
    }

    fn connect_home2(home_profile_id: &ProfileId, prof_repo: Rc<ProfileRepo>,
                     connector: Rc<HomeConnector>, signer: Rc<Signer>)
        -> Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let home_conn_fut = prof_repo.load(home_profile_id)
            .and_then( move |home_profile| connector.connect(&home_profile, signer) );
        Box::new(home_conn_fut)
    }


    pub fn login_home(&self, home_profile_id: ProfileId) ->
        Box< Future<Item=Rc<HomeSession>, Error=ErrorToBeSpecified> >
    {
        let home_id = home_profile_id.clone();
        let my_profile_id = self.signer.profile_id().to_owned();
        let login_fut = self.profile_repo.load(&my_profile_id)
            .and_then( |profile|
            {
                match profile.facet
                {
                    ProfileFacet::Persona(persona) => persona.homes.iter()
                        .filter(move |home_proof|
                            home_proof.peer_id(&my_profile_id)
                                .and_then(|peer_id| if *peer_id == home_id { Ok(true) } else { Err(ErrorToBeSpecified::TODO(String::new())) })
                                .is_ok()
                        )
                        .map( |home_proof| home_proof.to_owned() )
                        .nth(0)
                        .ok_or(ErrorToBeSpecified::TODO("login_home(): no proof found for specified home".to_string())),

                    _ => Err(ErrorToBeSpecified::TODO("login_home(): only persona profiles can log in".to_string()))
                }
            } )
            .and_then(
            {
                let profile_repo_clone = self.profile_repo.clone();
                let home_connector_clone = self.home_connector.clone();
                let signer_clone = self.signer.clone();
                move |home_proof| Self::connect_home2(&home_profile_id, profile_repo_clone, home_connector_clone, signer_clone)
                    .and_then( move |home| home.login(&home_proof) )
            } );
        Box::new(login_fut)
    }


    pub fn any_home_of(&self, profile: &Profile) ->
        Box< Future<Item=(RelationProof, Rc<Home>), Error=ErrorToBeSpecified> >
    {
        ProfileGatewayImpl::any_home_of2( profile, self.profile_repo.clone(),
                                          self.home_connector.clone(), self.signer.clone() )
    }


    fn any_home_of2(profile: &Profile, prof_repo: Rc<ProfileRepo>,
                    connector: Rc<HomeConnector>, signer: Rc<Signer>) ->
        Box< Future<Item=(RelationProof, Rc<Home>), Error=ErrorToBeSpecified> >
    {
        let homes = match profile.facet {
            // TODO consider how to get homes/addresses for apps and smartfridges
            ProfileFacet::Persona(ref facet) => facet.homes.clone(),
            _ => return Box::new(future::err(ErrorToBeSpecified::TODO("any_home_of: not a home profile".to_owned()))),
        };

        let home_conn_futs = homes.iter()
            .map( move |home_proof| //|home_id_res|
            {
                let prof_repo = prof_repo.clone();
                let connector = connector.clone();
                let signer = signer.clone();
                let proof = home_proof.to_owned();
                match home_proof.peer_id( signer.profile_id() ) {
                    Ok(ref home_id) => Box::new(
                        Self::connect_home2(home_id.to_owned(), prof_repo, connector, signer)
                            .map( move |home| (proof, home) )
                        ) as Box< Future<Item=(RelationProof, Rc<Home>), Error=ErrorToBeSpecified> >,
                    Err(e) => Box::new( future::err(e) ),
                }
            } )
            .collect::<Vec<_>>();

        // NOTE needed because select_ok() panics for empty lists instead of simply returning an error
        if home_conn_futs.len() == 0
            { return Box::new( future::err(ErrorToBeSpecified::TODO(String::from("ProfileGateway.any_home_of2 found no homes"))) ) }

        // Pick first successful home connection
        let result = future::select_ok(home_conn_futs)
            .map( |(home_conn, _pending_conn_futs)| home_conn );
        Box::new(result)
    }
}


impl ProfileGateway for ProfileGatewayImpl
{
    fn signer(&self) -> &Signer { &*self.signer }


    fn relations(&self) -> Box< Future<Item=Vec<Relation>, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO(String::from("ProfileGateway.relations "))) )
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
        let half_proof = RelationHalfProof::new("home", &home_id, &*self.signer);
        let reg_fut = self.connect_home(&home_id)
            .map_err( move |e| (own_prof_clone, e) )
            .and_then( move |home| home.register(own_prof, half_proof, invite) );
        Box::new(reg_fut)
    }


    fn update(&self, home_id: ProfileId, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let own_profile_clone = own_prof.clone();
        let upd_fut = self.login_home(home_id)
            .and_then( move |session| session.update(own_profile_clone) );
        Box::new(upd_fut)
    }


    fn unregister(&self, home_id: ProfileId, own_prof: ProfileId, newhome_id: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let unreg_fut = self.login_home(home_id)
            .and_then( move |session| session.unregister(newhome_id) );
        Box::new(unreg_fut)
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
                //let half_proof = ProfileGatewayImpl::new_half_proof(rel_type_clone.as_str(), &profile.id, signer_clone.clone() );
                let half_proof = RelationHalfProof::new(&rel_type_clone, &profile.id, &*signer_clone.clone() );
                ProfileGatewayImpl::any_home_of2(&profile, profile_repo_clone, home_connector_clone, signer_clone)
                    .and_then( move |(_home_proof, home)| home.pair_request(half_proof) )
            } );

        Box::new(pair_fut)
    }


    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let pair_fut = self.any_home_of(&rel.peer)
            .and_then( move |(_home_proof, home)| home.pair_response(rel.proof) );
        Box::new(pair_fut)
    }


    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame,
            to_caller: Option<AppMsgSink>) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
    {
        let call_fut = self.any_home_of(&rel.peer)
            .and_then( move |(_home_proof, home)|
                home.call(app, CallRequestDetails { relation: rel.proof,
                    init_payload: init_payload, to_caller: to_caller } ) ) ;
        Box::new(call_fut)
    }


    // TODO this should try connecting to ALL of our homes
    fn login(&self) -> Box< Future<Item=Rc<HomeSession>, Error=ErrorToBeSpecified> >
    {
        let log_fut = self.profile_repo.load( self.signer.profile_id() )
            .and_then( {
                let profile_repo_clone = self.profile_repo.clone();
                let home_conn_clone = self.home_connector.clone();
                let signer_clone = self.signer.clone();
                move |profile| ProfileGatewayImpl::any_home_of2(
                    &profile, profile_repo_clone, home_conn_clone, signer_clone)
            } )
            .and_then( move |(home_proof, home)| home.login(&home_proof) ) ;

        Box::new(log_fut)
    }
}
