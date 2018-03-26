#![allow(unused)]
use super::*;
use mercury_common::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use futures::{Async, Future, IntoFuture, Sink, Stream};

fn generate_hash( base : &str) -> Vec<u8> {
    encode(Hash::SHA2256, base.as_bytes()).unwrap()
}

fn generate_hash_from_vec( base : Vec<u8>) -> Vec<u8> {
    encode(Hash::SHA2256, &base).unwrap()
}

pub struct Signo{
    prof_id : ProfileId,
    pubkey : PublicKey,
    privkey : Vec<u8>,
}

impl Signo{
    pub fn new( whatever : &str)->Self{
        Signo{
            prof_id : ProfileId("MockSigner".as_bytes().to_owned()),
            pubkey : PublicKey(generate_hash_from_vec(generate_hash(whatever))),
            privkey : generate_hash(whatever),
        }
    }
}

impl Signer for Signo{
    fn prof_id(&self) -> &ProfileId{
        &self.prof_id
    }
    fn pub_key(&self) -> &PublicKey{
        &self.pubkey
    }
    fn sign(&self, data: Vec<u8>) -> Signature{
        let mut sig = String::new();
        sig.push_str( std::str::from_utf8(&data).unwrap() );
        sig.push_str( std::str::from_utf8(&self.privkey).unwrap() );
        Signature( sig.into_bytes() )
    }
}

pub struct DummyHomeConnector{
    pub home : DummyHome,
}
impl HomeConnector for DummyHomeConnector{
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >{
            unimplemented!();
            //Box::new(futures::future::ok(Rc::new(&self.home)))
        }
}

pub struct DummyHome {
    pub signer : Signo,
    pub ping_reply: String,
}

impl DummyHome {
    pub fn new(ping_reply: &str) -> DummyHome {
        DummyHome {
            signer: Signo::new("Mockarony"),
            ping_reply: String::from(ping_reply),
        }
    }
}

impl Future for DummyHome{
    type Item = DummyHome;
    type Error =ErrorToBeSpecified;
    fn poll(
        &mut self
    ) -> Result<Async<Self::Item>, Self::Error>{
        unimplemented!();
    }
}
impl PeerContext for DummyHome {
    fn my_signer(&self) -> &Signer {
        &self.signer
    }
    fn peer(&self) -> Option<Profile>{
        None
    }
    fn peer_pubkey(&self) -> Option<PublicKey>{
        None
    }
}

impl ProfileRepo for DummyHome{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    // TODO notifications on profile updates should be possible
}

impl Home for DummyHome{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
            unimplemented!()
        }

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    fn pair_response(&self, rel: Relation) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

    fn call(&self, rel: Relation, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >{
            unimplemented!()
        }

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate_signo(){
        let _signo = Signo();
        assert_eq!(3, 4);
    }
}
