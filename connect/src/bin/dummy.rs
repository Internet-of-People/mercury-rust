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


pub struct Dummy{
    home_id   : ProfileId, 
    prof_repo : HashMap<Vec<u8>, Profile>,
}

impl Dummy{
    pub fn new() -> Self {
        let mut prof_repo : HashMap<Vec<u8>, Profile>=HashMap::new();
        prof_repo.insert(Vec::from("Home"), mock::make_home_profile("/ip4/127.0.0.1/udp/9876","Home","HomeKey") );
        Self{
            home_id   : ProfileId( Vec::from( "DummyHome" ) ),
            prof_repo : prof_repo,
        }
    }
}

impl ProfileRepo for Dummy{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
    Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >{
        //Box::new( future::err(ErrorToBeSpecified::TODO) );
        unimplemented!();
    }

    fn load(&self, id: &ProfileId) ->
    Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
        let prof = self.prof_repo.get(&id.0);
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

impl Home for Dummy
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
        let ins = self.prof_repo.insert( own_prof.profile.id.0.clone(), own_prof.profile.clone() );
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
    pub fn new()->Self{
        Self{home: Rc::new(Dummy::new())}
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
    let dummy = Dummy::new();
}