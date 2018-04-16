#![allow(unused)]
extern crate mercury_connect;
extern crate mercury_home_protocol;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;

use mercury_connect::*;
use mercury_home_protocol::*;
use ::net::*;
use ::mock::*;

use futures::sync::mpsc;

use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write, stdin};

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};
use futures::{future, sync, Future, Stream};


pub struct ProfileStore{
    content : HashMap<ProfileId, Profile>,
}

impl ProfileStore{
    pub fn new() -> Self {
        ProfileStore{
            content : HashMap::new(),
        }
    }
}

impl ProfileStore{
    pub fn insert(&mut self, id : ProfileId, profile : Profile) -> Option<Profile>{
        println!("ProfileStore.add {:?}", id.0);
        self.content.insert(id, profile)
    }

    pub fn get( &self, id : ProfileId ) -> Option<&Profile>{
        self.content.get(&id)
    }
}

impl ProfileRepo for ProfileStore{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    Box< HomeStream<Profile, String> >{
        println!("ProfileStore.list");
        Box::new( futures::stream::empty() )
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("ProfileStore.load {:?}", id.0);
        let prof = self.content.get(&id);
        match prof {
            Some(profile) => {println!("ProfileStore.load.success");Box::new( future::ok(profile.to_owned()) )},
            None => Box::new( future::err(ErrorToBeSpecified::TODO) ),
        }
    }

    fn resolve(&self, url: &str) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("ProfileStore.resolve");
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

}

pub struct MyDummyHome{
    pub home_profile   : Profile, 
    pub prof_repo : Rc<ProfileStore>,
}

impl MyDummyHome{
    pub fn new(profile : Profile, dht : Rc<ProfileStore>) -> Self {
        println!("MyDummyHome.new");
        MyDummyHome{
            home_profile   : profile,
            prof_repo : dht,
        }
    }
    
    fn insert(&mut self, id : ProfileId, profile : Profile)->Option<Profile>{
        println!("MyDummyHome.insert");
        Rc::get_mut(&mut self.prof_repo).unwrap().insert(id, profile)
    }
}

impl ProfileRepo for MyDummyHome{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    Box< Stream<Item=Result<Profile, String>, Error=()> >{
        println!("MyDummyHome.list");
        Box::new( futures::stream::empty() )
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.load");
        let prof = self.prof_repo.get(id.to_owned());
        match prof {
            Some(profile) => Box::new( future::ok(profile.to_owned()) ),
            None => Box::new( future::err(ErrorToBeSpecified::TODO) ),
        }
    }

    fn resolve(&self, url: &str) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.resolve");
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

}

impl Home for MyDummyHome
{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
    Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.claim");
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&mut self, mut own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
    Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
        //make some relation magic
        //match own_prof.profile.facets[0].homes.append(dummy_relation(self.home_id));
        println!("MyDummyHome.register {:?}", own_prof);
        let mut ret : Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> > = Box::new( future::err( (own_prof.clone(), ErrorToBeSpecified::TODO) ) );
        let id = own_prof.profile.id.clone();
        let profile = own_prof.profile.clone();
        let mut own_profile = own_prof.clone();
        let mut storing = false;
        for mut facet in own_profile.profile.facets.iter_mut(){
            match facet {
                &mut ProfileFacet::Persona(ref mut persona) => {
                    persona.homes.append( &mut vec!(dummy_relation_proof() ) );
                    storing = true;
                },
                _ => {
                    ret = Box::new( future::err( (own_prof.clone(), ErrorToBeSpecified::TODO) ) );
                },
            };
        };

        if storing{
            let ins = self.insert( id.clone(), profile.clone() );
            println!("inserting: {:?}", ins);
            match ins{
                Some(updated) => {
                    ret = Box::new( future::err( (own_prof.clone(), ErrorToBeSpecified::TODO) ) );
                },
                None => {
                    println!("MyDummyHome.register.success");
                    ret = Box::new(future::ok(own_profile.clone()));
                },
            }
        }
        ret
        //own_prof.priv_data = Vec::from("potato");
    }

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
    Box< Future<Item=Box<mercury_home_protocol::HomeSession>, Error=ErrorToBeSpecified> >{
        println!("MyDummyHome.login");
        let session = HomeSessionDummy::new( Rc::clone(&self.prof_repo) );
        //Box::new( future::ok( Box::new( session ) ) )
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
    Box< Future<Item=(), Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    fn pair_response(&self, rel: RelationProof) ->
    Box< Future<Item=(), Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    fn call(&self, rel: RelationProof, app: ApplicationId, init_payload: AppMessageFrame) ->
    Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}
 
pub struct DummyConnector{
    home : Rc<RefCell<Home>>
}
impl DummyConnector{
    // pub fn new()->Self{
    //     Self{home: Rc::new(MyDummyHome::new())}
    // }

    pub fn new_with_home(home : Rc<RefCell<Home>>)->Self{
        println!("DummyConnector.new_with_home");
        Self{home: home}
    }
}
impl HomeConnector for DummyConnector{
    /// Initiate a permanent connection to the home server defined by `home_profile`, or return an
    /// existing, live `Home` immediately.
    /// `home_profile` must have a HomeFacet with at least an address filled in.
    /// `signer` belongs to me.
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<RefCell<Home>>, Error=ErrorToBeSpecified> >{
            println!("DummyConnector.connect");
            Box::new( future::ok( Rc::clone( &self.home ) ) )
    }
}

pub struct HomeSessionDummy
{
    repo : Rc<ProfileStore>
}


impl HomeSessionDummy
{
    pub fn new( repo : Rc<ProfileStore> ) -> Self{ 
        println!("HomeSessionDummy.new");
        Self{ repo : repo } 
    }
}


impl HomeSession for HomeSessionDummy
{
    fn update(&self, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        println!("HomeSessionDummy.update");
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    // TODO is the ID of the new home enough here or do we need the whole profile?
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO close/drop session connection after successful unregister()
        println!("HomeSessionDummy.unregister");
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }


    fn events(&self) -> Box< HomeStream<ProfileEvent, String> >
    {
        println!("HomeSessionDummy.events");
        let (sender, receiver) = sync::mpsc::channel(0);
        Box::new(receiver)
    }

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< HomeStream<Call, String> >
    {
        println!("HomeSessionDummy.checkin_app");
        let (sender, receiver) = sync::mpsc::channel(0);
        Box::new(receiver)
    }

    // TODO remove this after testing
    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >
    {
        println!("Ping received `{}`, sending it back", txt);
        Box::new( future::ok( txt.to_owned() ) )
    }
}

fn main(){
    // let dummy = MyDummyHome::new();
}
