extern crate bincode;
extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
extern crate ed25519_dalek;
extern crate futures;
extern crate multiaddr;
extern crate multihash;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate signatory;
extern crate tokio_core;

use std::rc::Rc;

use bincode::serialize;
use futures::{Future, sync::mpsc};
use multiaddr::Multiaddr;
use crypto::{ProfileValidator, SignatureValidator};


pub mod mercury_capnp;
pub mod crypto;


// TODO
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ErrorToBeSpecified { TODO(String) }


#[derive(Serialize, PartialEq, PartialOrd, Eq, Clone, Debug, Hash)]
pub struct ProfileId(pub Vec<u8>); // NOTE multihash::Multihash::encode() output

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PublicKey(pub Vec<u8>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PrivateKey(pub Vec<u8>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Signature(pub Vec<u8>);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Bip32Path(String);



pub trait Seed
{
    // TODO do we need a password to unlock the private key?
    fn unlock(bip32_path: &Bip32Path) -> Rc<Signer>;
}


/// Something that can sign data, but cannot give out the private key.
/// Usually implemented using a private key internally, but also enables hardware wallets.
pub trait Signer
{
    // TODO consider if this is needed here or should only be provided by trait PeerContext?
    fn prof_id(&self) -> &ProfileId;

    fn pub_key(&self) -> &PublicKey;
    // NOTE the data to be signed ideally will be the output from Mudlee's multicodec lib
    fn sign(&self, data: &[u8]) -> Signature;
}


pub trait Validator: ProfileValidator + SignatureValidator
{
    fn validate_half_proof(&self, half_proof: &RelationHalfProof, signer_public_key: &PublicKey) -> Result<(), ErrorToBeSpecified> {
        let signable_part = RelationSignablePart {
            relation_type: half_proof.relation_type.clone(),
            signer_id: half_proof.signer_id.clone(),
            peer_id: half_proof.peer_id.clone(),
        };

        self.validate_signature(signer_public_key, &signable_part.serialized(), &half_proof.signature)?;
        Ok(())
    }

    fn validate_relation_proof(
        &self,
        relation_proof: &RelationProof,
        id_1: &ProfileId,
        public_key_1: &PublicKey,
        id_2: &ProfileId,
        public_key_2: &PublicKey
    ) -> Result<(), ErrorToBeSpecified> {

        let signable_a = RelationSignablePart {
            relation_type: relation_proof.relation_type.clone(),
            signer_id: relation_proof.a_id.clone(),
            peer_id: relation_proof.b_id.clone(),
        }.serialized();

        let signable_b = RelationSignablePart {
            relation_type: relation_proof.relation_type.clone(),
            signer_id: relation_proof.b_id.clone(),
            peer_id: relation_proof.a_id.clone(),
        }.serialized();
        // TODO unwrap() can fail here in some special cases: when there is a limit set and it's exceeded - or when .len() is
        //      not supported for the types to be serialized. Neither is possible here, so the unwrap will not fail.
        //      But anyway this serialization will be swapped with something that in the first place cannot fail at all.

        let peer_of_id_1 = relation_proof.peer_id(&id_1)?;
        if peer_of_id_1 != id_2 {return Err(ErrorToBeSpecified::TODO("The relation does not contain both id_1 and id_2".to_owned()));}

        if *peer_of_id_1 == relation_proof.b_id {
            // id_1 is 'proof.id_a'
            self.validate_signature(&public_key_1, &signable_a, &relation_proof.a_signature)?;
            self.validate_signature(&public_key_2, &signable_b, &relation_proof.b_signature)?;
        } else {
            // id_1 is 'proof.id_b'
            self.validate_signature(&public_key_1, &signable_b, &relation_proof.b_signature)?;
            self.validate_signature(&public_key_2, &signable_a, &relation_proof.a_signature)?;
        }

        Ok(())
    }
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct PersonaFacet
{
    // TODO should we use only a RelationProof here instead of full Relation info?
    /// `homes` contain items with `relation_type` "home", with proofs included.
    /// Current implementation supports only a single home stored in `homes[0]`,
    /// Support for multiple homes will be implemented in a future release.
    pub homes:  Vec<RelationProof>,
    pub data:   Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeFacet
{
    /// Addresses of the same home server. A typical scenario of multiple addresses is when there is
    /// one IPv4 address/port, one onion address/port and some IPv6 address/port pairs.
    pub addrs:  Vec<Multiaddr>,
    pub data:   Vec<u8>,
}

// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct ApplicationFacet
{
    /// unique id of the application - like 'iop-chat'
    pub id:     ApplicationId,
    pub data:   Vec<u8>,
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RawFacet
{
    pub data: Vec<u8>, // TODO or maybe multicodec output?
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum ProfileFacet
{
    Home(HomeFacet),
    Persona(PersonaFacet),
    Application(ApplicationFacet),
    Unknown(RawFacet),
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Profile
{
    /// The Profile ID is a hash of the public key, similar to cryptocurrency addresses.
    pub id:         ProfileId,

    /// Public key used for validating the identity of the profile.
    pub pub_key:    PublicKey,
    pub facets:     Vec<ProfileFacet>, // TODO consider redesigning facet Rust types/storage
    // TODO consider having a signature of the profile data here
}

impl Profile
{
    pub fn new(id: &ProfileId, pub_key: &PublicKey, facets: &[ProfileFacet]) -> Self
        { Self{ id: id.to_owned(), pub_key: pub_key.to_owned(), facets: facets.to_owned() } }

    pub fn new_home(id: ProfileId, pub_key: PublicKey, address: Multiaddr) -> Self {

        let facet = HomeFacet {
            addrs: vec![address],
            data: vec![],
        };

        Self {
            id,
            pub_key,
            facets: vec![ProfileFacet::Home(facet)]
        }
    }
}


/// Represents a connection to another Profile (Home <-> Persona), (Persona <-> Persona)
pub trait PeerContext
{
    fn my_signer(&self) -> &Signer;
    fn peer_pubkey(&self) -> &PublicKey;
    fn peer_id(&self) -> &ProfileId;

    fn validate(&self, validator: &Validator) -> Result<(),ErrorToBeSpecified>
    {
        validator.validate_profile( self.peer_pubkey(), self.peer_id() )
            .and_then( |valid|
                if valid { Ok( () ) }
                else { Err( ErrorToBeSpecified::TODO( "Peer context is invalid".to_owned() ) ) } )
    }
}



pub type HomeStream<Elem, RemoteErr> = mpsc::Receiver< Result<Elem, RemoteErr> >;
pub type HomeSink<Elem, RemoteErr>   = mpsc::Sender< Result<Elem, RemoteErr> >;

/// Potentially a whole network of nodes with internal routing and sharding
pub trait ProfileRepo
{
    /// List all profiles that can be load()'ed or resolve()'d.
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        HomeStream<Profile,String>;

    /// Look for specified `id` and return. This might involve searching for the latest version
    /// of the profile in the dht, but if it's the profile's home server, could come from memory, too.
    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    /// Same as load(), but also contains hints for resolution, therefore it's more efficient than load(id)
    ///
    /// The `url` may contain
    /// * ProfileID (mandatory)
    /// * some profile metadata (for user experience enhancement) (big fat warning should be thrown if it does not match the latest info)
    /// * ProfileID of its home server
    /// * last known multiaddress(es) of its home server
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >;

    // TODO notifications on profile updates should be possible
}



#[derive(PartialEq, Eq, Clone, Debug)]
pub struct OwnProfile
{
    /// The public part of the profile. In the current implementation it must contain a single PersonaFacet.
    pub profile:    Profile,

    /// Hierarchical, json-like data structure, encoded using multicodec library,
    /// encrypted with the persona's keys, and stored on the home server
    pub priv_data:  Vec<u8>, // TODO maybe multicodec output?
}

impl OwnProfile
{
    pub fn new(profile: &Profile, private_data: &[u8]) -> Self
        { Self{ profile: profile.clone(), priv_data: private_data.to_owned() } }
}

#[derive(Serialize)]
pub struct RelationSignablePart {
    // the binary blob to be signed is rust-specific: Strings are serialized to a u64 (size) and the encoded string itself.
    pub relation_type: String,
    pub signer_id: ProfileId,
    pub peer_id: ProfileId,
}

impl RelationSignablePart {
    fn serialized(&self) -> Vec<u8> {
        // TODO unwrap() can fail here in some special cases: when there is a limit set and it's exceeded - or when .len() is
        //      not supported for the types to be serialized. Neither is possible here, so the unwrap will not fail.
        //      But still, to be on the safe side, this serialization shoule be swapped later with a call that cannot fail.
        Vec::from(serialize(self).unwrap())
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RelationHalfProof
{
    pub relation_type:  String,
    pub signer_id:      ProfileId,
    pub peer_id:        ProfileId,
    pub signature:      Signature,
    // TODO is a nonce needed?
}

impl RelationHalfProof
{
    // TODO add params and properly initialize
    pub fn new() -> Self
        { Self{ relation_type: String::new(), signer_id: ProfileId(Vec::new()),
                signature: Signature(Vec::new()), peer_id: ProfileId(Vec::new()) } }

    pub fn from_signable_part(signable_part: RelationSignablePart, signer: Rc<Signer>) -> Self {
        let signature = signer.sign(&serialize(&signable_part).unwrap());  // TODO remove unwrap(), investigate how it can fail

        RelationHalfProof {
            relation_type: signable_part.relation_type,
            signer_id: signable_part.signer_id,
            peer_id: signable_part.peer_id,
            signature: signature,
        }
    }
}


#[derive(PartialEq, Eq, Clone, Debug)]
pub struct RelationProof
{
    pub relation_type:  String,        // TODO inline halfproof fields with macro, if possible at all
    pub a_id:           ProfileId,
    pub a_signature:    Signature,
    pub b_id:           ProfileId,
    pub b_signature:    Signature,
    // TODO is a nonce needed?

}

impl RelationProof
{
    pub fn new(rel_type: &str, a_id: &ProfileId, a_signature: &Signature, b_id: &ProfileId, b_signature: &Signature) -> Self {
        if a_id < b_id {
            Self {
                relation_type: rel_type.to_owned(),
                a_id: a_id.to_owned(),
                a_signature: a_signature.to_owned(),
                b_id: b_id.to_owned(),
                b_signature: b_signature.to_owned(),
            }
        } else {
            Self {
                relation_type: rel_type.to_owned(),  // TODO decide which relation_type belongs here (`a_is_home_of_b` or `b_is_home_of_a`)
                a_id: b_id.to_owned(),
                a_signature: b_signature.to_owned(),
                b_id: a_id.to_owned(),
                b_signature: a_signature.to_owned(),
            }
        }
    }

    pub fn from_halfproof(half_proof: RelationHalfProof, peer_signature: Signature) -> Self
    {
        Self::new(half_proof.relation_type.as_ref(), &half_proof.signer_id, &half_proof.signature, &half_proof.peer_id, &peer_signature)
    }

    pub fn sign_halfproof(half_proof: RelationHalfProof, signer: &Signer) -> Self
    {
        let signable_part = RelationSignablePart {
            relation_type: half_proof.relation_type.clone(),
            signer_id: half_proof.peer_id.clone(),
            peer_id: half_proof.signer_id.clone(),
        };
        let signable_data = serialize(&signable_part).unwrap();  // TODO change to an implementation that cannot fail
        let home_signature = signer.sign(&signable_data);
        Self::from_halfproof(half_proof, home_signature)
    }

    pub fn peer_id(&self, my_id: &ProfileId) -> Result<&ProfileId, ErrorToBeSpecified> {
        if self.a_id == *my_id {
            Ok(&self.b_id)
        } else if self.b_id == *my_id {
            Ok(&self.a_id)
        } else {
            Err(ErrorToBeSpecified::TODO(format!("{:?} is not present in relation {:?}", my_id, self)))
        }
    }

    pub fn peer_signature(&self, my_id: &ProfileId) -> Result<&Signature, ErrorToBeSpecified> {
        if self.a_id == *my_id {
            Ok(&self.b_signature)
        } else if self.b_id == *my_id {
            Ok(&self.a_signature)
        } else {
            Err(ErrorToBeSpecified::TODO(format!("{:?} is not present in relation {:?}", my_id, self)))
        }
    }
}

/// This invitation allows a persona to register on the specified home.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct HomeInvitation
{
    pub home_id:    ProfileId,

    /// A unique string that identifies the invitation
    pub voucher:    String,

    /// The signature of the home
    pub signature:  Signature,
    // TODO is a nonce needed?
    // TODO is an expiration time needed?
}

impl HomeInvitation
{
    pub fn new(home_id: &ProfileId, voucher: &str, signature: &Signature) -> Self
        { Self{ home_id: home_id.to_owned(), voucher: voucher.to_owned(), signature: signature.to_owned() } }
}


#[derive(PartialEq, Eq, Clone, Debug, Hash)]
pub struct ApplicationId(pub String);

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct AppMessageFrame(pub Vec<u8>);


pub type AppMsgStream = HomeStream<AppMessageFrame, String>;
pub type AppMsgSink   = HomeSink<AppMessageFrame, String>;


#[derive(Debug)]
pub struct CallRequestDetails
{
    pub relation:       RelationProof,
    pub init_payload:   AppMessageFrame,
    // NOTE A missed call or p2p connection failure will result Option::None
    pub to_caller:      Option<AppMsgSink>,
}


// Interface to a single home server.
// NOTE authentication is already done when the connection is built,
//      authenticated profile info is available from the connection context
pub trait Home: ProfileRepo
{
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >;

    // TODO consider how to enforce overwriting the original ownprofile with the modified one
    //      with the pairing proof, especially the error case
    fn register(&self, own_prof: OwnProfile, half_proof: RelationHalfProof, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >;

    // TODO decide: login() takes a ProfileId parameter because the hashing algorithm, which
    //              we use to create a profile id from a public key is not fixed. We use multihash,
    //              which means we pick one algorithm for now, and when we consider it insecure, we
    //              use another one. Let's say it takes 5 years to break our current hashing algorithm.
    //              This means for the first 5 years we don't have to guess, and in the next 5 years
    //              we could start guessing with the new algorithm, and fall-back to the deprecated
    //              algorithm. This does not involve much performance neither complexity to the server code.
    //
    //              However, the `profile` parameter increases learning curve for the API by a small amount.
    //              Newcomers might raise (stupid, but without prior knowledge, reasonable) questions like these:
    //               * Why do I need to specify my ProfileId if it was already specified during authentication?
    //               * Why do I need to specify my ProfileId if it can be calculated from my public key I just used?
    //               * If this is a login, and we provide the credential, where is the password?
    //
    //              Since we would like to provide the most simple api possible, my suggestion is to rename
    //              this function to `start_session(&self)` and remove the ProfileId parameter.

    // NOTE this closes all previous sessions of the same profile
    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Rc<HomeSession>, Error=ErrorToBeSpecified> >;


    // NOTE acceptor must have this server as its home
    // NOTE empty result, acceptor will connect initiator's home and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    fn pair_response(&self, rel: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // NOTE initiating a real P2P connection (vs a single frame push notification),
    //      the caller must fill in some message channel to itself.
    //      A successful call returns a channel to callee.
    fn call(&self, app: ApplicationId, call_req: CallRequestDetails) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >;

// TODO consider how to do this in a later milestone
//    fn presence(&self, rel: Relation, app: ApplicationId) ->
//        Box< Future<Item=Option<AppMessageFrame>, Error=ErrorToBeSpecified> >;
}


#[derive(Clone)]
pub enum ProfileEvent
{
    Unknown(Vec<u8>), // forward compatibility for protocol extension
    PairingRequest(RelationHalfProof),
    PairingResponse(RelationProof),
// TODO are these events needed? What others?
//    HomeBroadcast,
//    HomeHostingExpiry,
//    ProfileUpdated, // from a different client instance/session
}


pub trait IncomingCall
{
    fn request_details(&self) -> &CallRequestDetails;
    // NOTE this assumes boxed trait objects, if Rc of something else is needed, this must be revised
    // TODO consider offering the possibility to somehow send back a single AppMessageFrame
    //      as a reply to init_payload without a to_callee sink,
    //      either included into this function or an additional method
    fn answer(self: Box<Self>, to_callee: Option<AppMsgSink>);
}

pub trait HomeSession
{
    fn update(&self, own_prof: OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    // TODO should we return a modified OwnProfile here with this home removed from the homes of persona facet in profile?
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >;


    fn events(&self) -> HomeStream<ProfileEvent, String>;

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> HomeStream<Box<IncomingCall>, String>;

    // TODO remove this after testing
    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >;


// TODO ban features are delayed to a later milestone
//    fn banned_profiles(&self) ->
//        Box< Future<Item=Vec<ProfileId>, Error=ErrorToBeSpecified> >;
//
//    fn ban(&self, profile: &ProfileId) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
//
//    fn unban(&self, profile: &ProfileId) ->
//        Box< Future<Item=(), Error=ErrorToBeSpecified> >;
}



#[cfg(test)]
mod tests
{
    use futures::{Sink, Stream};
    use futures::sync::mpsc;
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

    #[test]
    fn test_mpsc_drop_receiver()
    {
        let mut setup = TestSetup::new();
        let (sender, receiver) = mpsc::channel(2);

        // Send and item
        let item = "Hello".to_owned();
        let send_fut = sender.send( item.clone() );
        let sender = setup.reactor.run(send_fut).unwrap();

        // Receive the sent item
        // NOTE take() drops the receiver after the first element
        let recv_fut = receiver.take(1).collect();
        let recv_vec = setup.reactor.run(recv_fut).unwrap();
        assert_eq!( recv_vec.len(), 1 );
        assert_eq!( recv_vec[0], item );

        // Further sends should fail
        let send_fut = sender.send(item);
        let sender = setup.reactor.run(send_fut);
        assert!( sender.is_err() );
    }


    #[test]
    fn test_mpsc_drop_sender()
    {
        let mut setup = TestSetup::new();
        let (sender, receiver) = mpsc::channel(2);

        // Send an item and drop the sender
        let item = "Hello".to_owned();
        let send_fut = sender.send( item.clone() );
        let sender = setup.reactor.run(send_fut).unwrap();
        drop(sender);

        // Consume the stream Collecting all received elements
        let recv_fut = receiver.collect();
        let recv_vec = setup.reactor.run(recv_fut).unwrap();

        // Stream must end after dropped sender
        assert_eq!( recv_vec.len(), 1 );
        assert_eq!( recv_vec[0], item );
    }
}
