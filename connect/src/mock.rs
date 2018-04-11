#![allow(unused)]
use super::*;

use multihash::{encode, Hash};
use multiaddr::{Multiaddr, ToMultiaddr};

use futures::{Async, Future, IntoFuture, Sink, Stream};

pub fn generate_hash( base : &str) -> Vec<u8> {
    encode(Hash::SHA2256, base.as_bytes()).unwrap()
}

pub fn generate_hash_from_vec( base : Vec<u8>) -> Vec<u8> {
    encode(Hash::SHA2256, &base).unwrap()
}

pub fn create_ownprofile(name : &str)->OwnProfile{
    let p = Profile{
        id:         ProfileId(name.as_bytes().to_owned()),
        pub_key:    PublicKey("publickey".as_bytes().to_owned()),
        facets:     vec!(),
    };
    OwnProfile::new(&p, &[])
}

pub fn make_own_persona_profile(name : &str, pubkey : &PublicKey)->Profile{
    let id = generate_hash(name);
    let empty = vec![];
    let homes = vec![];
    Profile::new(
        &ProfileId(id), 
        &pubkey,
        &[ProfileFacet::Persona( PersonaFacet{ homes : homes , data : empty } )] 
    )
}

pub fn make_home_profile(addr : &str, name : &str, pubkey : &str)->Profile{
    let homeaddr = addr.to_multiaddr().unwrap();
    let home_hash = mock::generate_hash(name);
    let empty = vec![];
    let homevec = vec![homeaddr];
    Profile::new(
        &ProfileId(home_hash), 
        &PublicKey(pubkey.as_bytes().to_owned()),
        &[ProfileFacet::Home( HomeFacet{ addrs : homevec , data : empty } ) ] 
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
    fn sign(&self, data: &[u8]) -> Signature{
        let mut sig = String::new();
        sig.push_str( std::str::from_utf8(&data).unwrap() );
        sig.push_str( std::str::from_utf8(&self.privkey).unwrap() );
        Signature( sig.into_bytes() )
    }
}
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct DummyHomeConnector{
    pub home : DummyHome,
}
impl DummyHomeConnector{
    fn dconnect(&self){
        unimplemented!();
    }
}
impl HomeConnector for DummyHomeConnector{
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >{
            println!("connect");
            unimplemented!();
            //Box::new(futures::future::ok(Rc::new(&self.home)))
        }
}
#[derive(PartialEq, Eq, Clone, Debug)]
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
        println!("poll");
        unimplemented!();
    }
}
impl PeerContext for DummyHome {
    fn my_signer(&self) -> &Signer {
        &self.signer
    }
    fn peer(&self) -> Option<Profile>{
        println!("peer");
        None
    }
    fn peer_pubkey(&self) -> Option<PublicKey>{
        println!("peer_pubkey");
        None
    }
}

impl ProfileRepo for DummyHome{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< HomeStream<Profile, String> >{
            println!("list");
            unimplemented!()
        }

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
            println!("load: {:?}" , id );
            unimplemented!()
        }

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >{
            println!("resolve");
            unimplemented!()
        }

    // TODO notifications on profile updates should be possible
}

impl Home for DummyHome{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >{
            println!("claim");
            unimplemented!()
        }

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&mut self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >{
            println!("register: {:?}", own_prof.profile.id);
            unimplemented!()
        }

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >{
            println!("login: {:?}", profile);
            unimplemented!()
        }


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >{
            println!("pair_request");
            unimplemented!()
        }

    fn pair_response(&self, rel: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >{
            println!("pair_response");
            unimplemented!()
        }

    fn call(&self, rel: RelationProof, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >{
            println!("call");
            unimplemented!()
        }

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}

pub fn dummy_half_proof(rtype: &str)->RelationHalfProof{
    RelationHalfProof{
        relation_type:  String::from(rtype),
        my_id:          ProfileId("my_id".as_bytes().to_owned()),
        my_sign:        Signature("my_sign".as_bytes().to_owned()),
        peer_id:        ProfileId("peer_id".as_bytes().to_owned()),
    }
}

pub fn dummy_relation_proof()->RelationProof {
    RelationProof::new()
}

pub fn dummy_relation(rtype: &str)->Relation{
    Relation::new(
        &make_own_persona_profile(
            "relation_profile",
            &PublicKey("dummy_relation_profile_id".as_bytes().to_owned()) 
        ),
        &dummy_relation_proof()
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn instantiate_signo(){
        let _signo = Signo::new("Trololo");
        assert_eq!(3, 4);
    }
}
