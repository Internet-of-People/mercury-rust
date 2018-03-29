#![allow(unused)]
extern crate mercury_sdk;
extern crate mercury_common;

extern crate multihash;
extern crate multiaddr;

extern crate tokio_stdin_stdout;
extern crate tokio_core;
extern crate tokio_io;
extern crate futures;

use mercury_common::*;
use mercury_sdk::*;
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


struct Dummy{
    prof_repo : HashMap<Vec<u8>, Profile>,
}

impl Dummy{
    pub fn new() -> Self {
        Self{
            prof_repo : HashMap::new()
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
    fn register(&self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
    Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
        Box::new( future::err( (own_prof, ErrorToBeSpecified::TODO) ) )
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

fn main(){
    let dummy = Dummy::new();
}