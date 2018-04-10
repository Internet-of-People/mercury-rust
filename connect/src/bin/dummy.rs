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

use std::rc::Rc;
use std::collections::HashMap;
use std::io::{BufRead, Read, Write, stdin};

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::{AsyncRead, AsyncWrite};
use futures::{future, Future,Stream};


pub struct Dht{
    content : HashMap<ProfileId, Profile>,
}

impl Dht{
    pub fn new() -> Self {
        Dht{
            content : HashMap::new(),
        }
    }
}

impl Dht{
    pub fn insert(&mut self, id : ProfileId, profile : Profile) -> Option<Profile>{
        println!("Dht.add {:?}", id.0);
        self.content.insert(id, profile)
    }

    pub fn get( &self, id : ProfileId ) -> Option<&Profile>{
        self.content.get(&id)
    }
}

impl ProfileRepo for Dht{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    Box< HomeStream<Profile, String> >{
        Box::new( futures::stream::empty() )
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        println!("Dht.load {:?}", id.0);
        let prof = self.content.get(&id);
        match prof {
            Some(profile) => {println!("dht.load.success");Box::new( future::ok(profile.to_owned()) )},
            None => Box::new( future::err(ErrorToBeSpecified::TODO) ),
        }
    }

    fn resolve(&self, url: &str) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

}

pub struct MyDummyHome{
    pub home_profile   : Profile, 
    pub prof_repo : Rc<Dht>,
}

impl MyDummyHome{
    pub fn new(profile : Profile, dht : Rc<Dht>) -> Self {
        MyDummyHome{
            home_profile   : profile,
            prof_repo : dht,
        }
    }
    
    pub fn insert(&mut self, id : ProfileId, profile : Profile)->Option<Profile>{
        Rc::get_mut(&mut self.prof_repo).unwrap().insert(id, profile)
    }
}

impl ProfileRepo for MyDummyHome{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    Box< Stream<Item=Result<Profile, String>, Error=()> >{
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
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

}

impl Home for MyDummyHome
{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
    Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >{
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&mut self, mut own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
    Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
        //make some relation magic
        //match own_prof.profile.facets[0].homes.append(dummy_relation(self.home_id));
        let mut ret : Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >;
        match own_prof.profile.facets[0] {
            ProfileFacet::Persona(ref mut persona) => {
                persona.homes.append( &mut vec!(dummy_relation_proof() ) );
            },
            _ => {
                /*Box::new( future::err( (own_prof, ErrorToBeSpecified::TODO) ) )*/
                //ret = Box::new(future::ok(own_prof));
            },
        };
        let ins = self.insert( own_prof.profile.id.clone(), own_prof.profile.clone() );
        println!("--------------------------------------------------------");
        println!("{:?}", ins);
        match ins{
            Some(updated) => {
                ret = Box::new( future::err( (own_prof, ErrorToBeSpecified::TODO) ) );
            } ,
            None => {
                ret = Box::new(future::ok(own_prof));
            },
        }
        ret
        //own_prof.priv_data = Vec::from("potato");
        
    }

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
    Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >{
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
    home : Rc<Home>
}
impl DummyConnector{
    // pub fn new()->Self{
    //     Self{home: Rc::new(MyDummyHome::new())}
    // }

    pub fn new_with_home(home : Rc<Home>)->Self{
        Self{home: home}
    }
}
impl HomeConnector for DummyConnector{
    /// Initiate a permanent connection to the home server defined by `home_profile`, or return an
    /// existing, live `Home` immediately.
    /// `home_profile` must have a HomeFacet with at least an address filled in.
    /// `signer` belongs to me.
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >{
            Box::new( future::ok( Rc::clone( &self.home ) ) )
    }
}


fn main(){
    // let dummy = MyDummyHome::new();
}
