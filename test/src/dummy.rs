#![allow(unused)]
extern crate mercury_home_protocol;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;
extern crate base64;

/*

use mercury_connect::*;
use mercury_home_protocol::*;

use super::*;
use mercury_connect::net::*;

use std::rc::Rc;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write, stdin};

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};

use futures::sync::mpsc;
use futures::{future, Async, sync, Future, IntoFuture, Sink, Stream};

pub fn generate_hash( base : &str) -> Vec<u8> {
    encode(Hash::SHA2256, base.as_bytes()).unwrap()
}

pub fn generate_hash_from_vec( base : Vec<u8>) -> Vec<u8> {
    encode(Hash::SHA2256, &base).unwrap()
}

pub struct TestSetup{
    pub reactor : tokio_core::reactor::Core,
    pub handle : tokio_core::reactor::Handle,
    pub homeprofile: Profile,
    pub homeprofileid: ProfileId,
    pub homesigner: Rc<Signer>,
    pub homeaddr: String,
    pub homemultiaddr: Multiaddr,
    pub home: Rc<MyDummyHome>,
    pub user: Profile,
    pub userid: ProfileId,
    pub usersinger: Rc<Signer>,
    pub userownprofile : OwnProfile, 
    pub profilegate: ProfileGatewayImpl,
}

impl TestSetup{
    pub fn setup()-> Self {

        let homeaddr = String::from("/ip4/127.0.0.1/udp/9876");
        let homemultiaddr = homeaddr.to_multiaddr().unwrap();
        let (homeprof, homesigner) = generate_profile(ProfileFacet::Home(HomeFacet{addrs: vec![homemultiaddr.clone().into()], data: vec![]}));

        let homeprofileid =  homeprof.id.clone();

        let (user, usersigner) = generate_profile(ProfileFacet::Persona(PersonaFacet{homes: vec![], data: vec![]}));
        let userid = user.id.clone();
        let userownprofile = create_ownprofile(user.clone());

        let mut dht = ProfileStore::new();
        dht.insert(homeprofileid.clone(), homeprof.clone());
        let mut home_storage = Rc::new(dht);
        let mut store_rc = Rc::clone(&home_storage);
        let mut home = Rc::new( MyDummyHome::new( homeprof.clone() , home_storage ) );
        let homerc = Rc::clone(&home);

        let usersigner = Rc::new(usersigner);
        let profilegateway = ProfileGatewayImpl{
            signer:         usersigner.clone(),
            profile_repo:   store_rc,
            home_connector: Rc::new( dummy::DummyConnector::new_with_home(home) ),
        };

        let reactor = tokio_core::reactor::Core::new().unwrap();
        let handle = reactor.handle();

        Self{
            reactor: reactor,
            handle: handle,
            homeprofile: homeprof,
            homeprofileid: homeprofileid,
            homesigner: Rc::new(homesigner),
            homeaddr: homeaddr,
            homemultiaddr: homemultiaddr,
            home: homerc,
            user: user,
            userid: userid,
            usersinger: usersigner,
            userownprofile: userownprofile,
            profilegate: profilegateway,
        }
    }
}


pub fn create_ownprofile(p : Profile)->OwnProfile{
    OwnProfile::new(&p, &[])
}

pub fn make_own_persona_profile(pubkey : &PublicKey)->Profile{
    let id = generate_hash_from_vec(pubkey.0.clone());
    let empty = vec![];
    let homes = vec![];
    Profile::new(
        &ProfileId(id), 
        &pubkey,
        &ProfileFacet::Persona( PersonaFacet{ homes : homes , data : empty } )
    )
}

pub fn make_home_profile(addr : &str, pubkey : &PublicKey)->Profile{
    let homeaddr = addr.to_multiaddr().unwrap();
    let home_hash = generate_hash_from_vec(pubkey.0.clone());
    let empty = vec![];
    let homevec = vec![homeaddr];
    Profile::new(
        &ProfileId(home_hash), 
        &pubkey,
        &ProfileFacet::Home( HomeFacet{ addrs : homevec , data : empty } )
    )
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Signo{
    prof_id : ProfileId,
    pubkey : PublicKey,
    privkey : Vec<u8>,
}

impl Signo{
    pub fn new( whatever : &str)->Self{
        Signo{
            prof_id : ProfileId( generate_hash_from_vec( generate_hash(whatever) ) ),
            pubkey : PublicKey( generate_hash(whatever) ),
            privkey : generate_hash(whatever),
        }
    }

    pub fn get_base64_id(&self)->String{
        base64::encode(&self.prof_id.0)
    }
}

pub fn dummy_relation(rel_type: &str) -> Relation{
    Relation::new(
        &make_own_persona_profile( &PublicKey( Vec::from( "too_hot_today" ) ) ),
        &dummy_relation_proof(rel_type)
    )
}

pub fn dummy_relation_proof(rel_type: &str)->RelationProof{
    RelationProof::new( 
        rel_type, 
        &ProfileId(Vec::from("TestMe")),
        &Signature(Vec::from("TestMe")),
        &ProfileId(Vec::from("TestOther")),
        &Signature(Vec::from("TestOther"))
        )
} 

#[derive(Debug)]
pub struct ProfileStore{
    content : RefCell< HashMap<ProfileId, Profile> >,
}

impl ProfileStore{
    pub fn new() -> Self {
        ProfileStore{
            content : RefCell::new( HashMap::new() ),
        }
    }
}

impl ProfileStore{
    pub fn insert(&self, id : ProfileId, profile : Profile) -> Option<Profile>{
        self.content.borrow_mut().insert(id, profile)
    }

    pub fn get( &self, id : ProfileId ) -> Option<Profile>{
        self.content.borrow().get(&id).map( |x| x.to_owned() )
    }
}

impl ProfileRepo for ProfileStore{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    HomeStream<Profile, String>{
        println!("ProfileStore.list");
        let (send, recv) = futures::sync::mpsc::channel(0);
        recv
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        let store = self.content.borrow();
        let prof = store.get(&id);
        match prof {
            Some(profile) => {
                Box::new( future::ok(profile.to_owned()) )},
            None => {
                println!("ProfileStore.load.fail"); 
                Box::new( future::err(ErrorToBeSpecified::TODO(String::from("ProfileStore/ProfileRepo.load "))) )},
        }
    }

    fn resolve(&self, url: &str) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("ProfileStore.resolve");
        println!("{:?}",Vec::from(url) );
        self.load(&ProfileId(Vec::from(url)))
        // match base64::decode(url){
        //     Ok(id)=> {
        //         self.load(&ProfileId(id))
        //         },
        //     Err(e)=> Box::new( future::err(ErrorToBeSpecified::TODO(String::from("ProfileStore/ProfileRepo.resolve "))) )
        // }
        
        //
    }

}

pub struct MyDummyHome{
    pub home_profile   : Profile, 
    pub local_prof_store : RefCell<HashMap<ProfileId, Vec<u8>>>,
    pub storage_layer : Rc<ProfileStore>,
    pub events : RefCell<HashMap<ProfileId, Vec<ProfileEvent>>>,
}

impl MyDummyHome{
    pub fn new(profile : Profile, dht : Rc<ProfileStore>) -> Self {
        MyDummyHome{
            home_profile : profile,
            local_prof_store : RefCell::new( HashMap::new() ),
            storage_layer : dht,
            events : RefCell::new( HashMap::new() )
        }
    }
    
    pub fn insert(&self, id : ProfileId, profile : Profile)->Option<Profile>{
        println!("MyDummyHome.insert");
        self.storage_layer.insert(id, profile)
    }
}

impl ProfileRepo for MyDummyHome {
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    HomeStream<Profile, String>{
        println!("MyDummyHome.list");
        let (send, recv) = futures::sync::mpsc::channel(0);
        recv
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.load");
        let prof = self.storage_layer.get(id.to_owned());
        match prof {
            Some(profile) => Box::new( future::ok(profile.to_owned()) ),
            None => Box::new( future::err(ErrorToBeSpecified::TODO(String::from("MyDummyHome.load "))) ),
        }
    }

    fn resolve(&self, url: &str) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.resolve");
        match base64::decode(url){
            Ok(id)=> {
                self.load(&ProfileId(id))
                },
            Err(e)=> Box::new( future::err(ErrorToBeSpecified::TODO(String::from("ProfileStore/ProfileRepo.resolve "))) )
        }
    }
 
}

impl Home for MyDummyHome
{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
    Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.claim");
        match self.storage_layer.get(profile.clone()){
            Some(own) => {
                match self.local_prof_store.borrow().get(&profile){
                        Some(privdata) => Box::new( future::ok( OwnProfile::new( &own, privdata ) ) ),
                        None => Box::new( future::ok( OwnProfile::new( &own, &Vec::new()) ) )
                }
            },
            None => Box::new( future::err( ErrorToBeSpecified::TODO( String::from( "MyDummyHome.claim" ) ) ) )
        }
    }

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&self, mut own_prof: OwnProfile, half_proof: RelationHalfProof, invite: Option<HomeInvitation>) ->
    Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
        println!("REGISTERING{:?}", own_prof.profile.id.0);
        let id = own_prof.profile.id.clone();
        let profile = own_prof.profile.clone();
        let mut own_profile = own_prof.clone();
        let mut storing = false;
        match own_profile.profile.facet {
            ProfileFacet::Persona(ref mut persona) => {
                let half_proof_clone = half_proof.clone();
//                    let relation_proof = RelationProof::from_halfproof(half_proof_clone, Signature(self.home_profile.public_key.0.clone()));
//                    persona.homes.append( &mut vec!(relation_proof ) );
                storing = true;
            },
            _ => {
                return Box::new( future::err( (own_prof.clone(),
                ErrorToBeSpecified::TODO(String::from("MyDummyHome.register fails at finding Persona Facet on given profile "))) ) );
            },
        };

        let mut ret : Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> > = Box::new( future::err( (own_prof.clone(), ErrorToBeSpecified::TODO(String::from("MyDummyHome.register had unknown error "))) ) );
        
        if storing{
            let ins = self.insert( id.clone(), own_profile.profile.clone() );
            match ins{
                Some(updated) => {
                    ret = Box::new( future::err( (own_prof.clone(), ErrorToBeSpecified::TODO(String::from("MyDummyHome.register already had given profile stored "))) ) );
                },
                None => {
                    println!("MyDummyHome.register.success");
                    self.local_prof_store.borrow_mut().insert(id, own_profile.priv_data.clone());
                    ret = Box::new(future::ok(own_profile.clone()));
                },
            }
        }
        ret
    }

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, proof: &RelationProof) ->
    Box< Future< Item=Rc<HomeSession>, Error=ErrorToBeSpecified > >{
        unimplemented!();
//        println!("MyDummyHome.login");
//        //let selfcell = Rc::new(RefCell::new(*self));
//        let session = Rc::new(HomeSessionDummy::new( profile.to_owned() ,Rc::clone(&self.storage_layer)/*, selfcell */) ) as Rc<HomeSession>;
//        Box::new( future::ok( session ) )
//        //Box::new( future::err(ErrorToBeSpecified::TODO(String::from("MyDummyHome.login "))) )

    } 


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
    Box< Future<Item=(), Error=ErrorToBeSpecified> >{
        let peer_id = half_proof.peer_id.clone();
        let req_event = ProfileEvent::PairingRequest(half_proof);
        match self.events.borrow_mut().entry(peer_id).or_insert(Vec::new()).push(req_event){
            () => Box::new( future::ok( () ) ),
            _ =>  Box::new( future::err(ErrorToBeSpecified::TODO(String::from("MyDummyHome.pair_request "))) )
        }
        //self.events.insert(half_proof.peer_id.clone(), profile_events;

    }

    fn pair_response(&self, rel_proof: RelationProof) ->
    Box< Future<Item=(), Error=ErrorToBeSpecified> >{
        let peer_id = rel_proof.peer_id(&self.home_profile.id).unwrap().clone();
        let resp_event = ProfileEvent::PairingResponse(rel_proof);
        match self.events.borrow_mut().entry(peer_id).or_insert(Vec::new()).push(resp_event){
            () => Box::new( future::ok( () ) ),
            _ =>  Box::new( future::err(ErrorToBeSpecified::TODO(String::from("MyDummyHome.pair_response "))) )
        }
    }

    fn call(&self, app: ApplicationId, call_req: CallRequestDetails) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("MyDummyHome.call "))) )
    }

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}
 
pub struct DummyConnector{
    home : Rc<Home>
}
impl DummyConnector{
    pub fn new_with_home(home : Rc<Home>)->Self{
        Self{home: home}
    }
}
impl mercury_connect::HomeConnector for DummyConnector{
    /// Initiate a permanent connection to the home server defined by `home_profile`, or return an
    /// existing, live `Home` immediately.
    /// `home_profile` must have a HomeFacet with at least an address filled in.
    /// `signer` belongs to me.
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >{
            println!("DummyConnector.connect");
            Box::new( future::ok( Rc::clone( &self.home ) ) )
    }
}

pub struct HomeSessionDummy
{
    prof : ProfileId,
    repo : Rc<ProfileStore>,
    //home : Rc< RefCell< MyDummyHome > >,
}


impl HomeSessionDummy
{
    pub fn new( prof : ProfileId, repo : Rc<ProfileStore>/*, home : Rc<RefCell<MyDummyHome>> */) -> Self{
        Self{ prof : prof, repo : repo/*, home : home */} 
    }
}


impl HomeSession for HomeSessionDummy
{
    fn update(&self, own_prof: OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        println!("HomeSessionDummy.update");
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionDummy.update "))) )
    }

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    // TODO is the ID of the new home enough here or do we need the whole profile?
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO close/drop session connection after successful unregister()
        println!("HomeSessionDummy.unregister");
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionDummy.unregister "))) )
    }


    fn events(&self) -> HomeStream<ProfileEvent, String>
    {
        println!("HomeSessionDummy.events");
        let (sender, receiver) = futures::sync::mpsc::channel(1);
        // &self.stream.push(sender);
        // &self.stream[0].send( Ok( ProfileEvent::Unknown( Vec::from("DummyEvents")) ) );

        receiver
        // match self.home.borrow().events.get(&self.prof){
        //     Some(evec) => {
        //         let event_vector = evec.to_owned();
        //         for e in event_vector {
        //             let event : mercury_home_protocol::ProfileEvent = e.to_owned();
        //             sender.send(Ok(event));
        //             }
        //         },
        //     None => {
        //         sender.send(Err(String::from("no events")));
        //         },
        // }
    }

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> HomeStream<Box<IncomingCall>, String>
    {
        println!("HomeSessionDummy.checkin_app");
        let (sender, receiver) = sync::mpsc::channel(0);
        receiver
    }

    // TODO remove this after testing
    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >
    {
        println!("Ping received `{}`, sending it back", txt);
        Box::new( future::ok( txt.to_owned() ) )
    }
}

pub struct Incall{
    request : CallRequestDetails,
}

impl IncomingCall for Incall{
    fn request_details(&self) -> &CallRequestDetails{
        &self.request
    }
    fn answer(self: Box<Self>, to_callee: Option<AppMsgSink>) -> CallRequestDetails { self.request }
}

*/