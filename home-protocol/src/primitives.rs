use serde::{de::Error as DeSerError, ser::SerializeSeq};
use serde::{Deserialize as DeSer, Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};

use bincode::serialize;
use multiaddr::{Multiaddr, ToMultiaddr};

use crate::*;

pub type ProfileId = keyvault::multicipher::MKeyId;
pub type PublicKey = keyvault::multicipher::MPublicKey;
pub type PrivateKey = keyvault::multicipher::MPrivateKey;
pub type Signature = keyvault::multicipher::MSignature;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct ApplicationId(pub String);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PersonaFacet {
    /// `homes` contain items with `relation_type` "home", with proofs included.
    /// Current implementation supports only a single home stored in `homes[0]`,
    /// Support for multiple homes will be implemented in a future release.
    pub homes: Vec<RelationProof>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HomeFacet {
    /// Addresses of the same home server. A typical scenario of multiple addresses is when there is
    /// one IPv4 address/port, one onion address/port and some IPv6 address/port pairs.
    #[serde(serialize_with = "serialize_multiaddr_vec")]
    #[serde(deserialize_with = "deserialize_multiaddr_vec")]
    pub addrs: Vec<Multiaddr>,
    pub data: Vec<u8>,
}

// NOTE Given for each SUPPORTED app, not currently available (checked in) app, checkins are managed differently
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Serialize)]
pub struct ApplicationFacet {
    /// unique id of the application - like 'iop-chat'
    pub id: ApplicationId,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, PartialOrd, Serialize)]
pub struct RawFacet {
    pub data: Vec<u8>, // TODO or maybe multicodec output?
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProfileFacet {
    Home(HomeFacet),
    Persona(PersonaFacet),
    Application(ApplicationFacet),
    Unknown(RawFacet),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Profile {
    /// The Profile ID is a hash of the public key, similar to cryptocurrency addresses.
    pub id: ProfileId,

    /// Public key used for validating the identity of the profile.
    pub public_key: PublicKey,
    pub facet: ProfileFacet, // TODO consider redesigning facet Rust types/storage
                             // TODO consider having a signature of the profile data here
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OwnProfile {
    /// The public part of the profile. In the current implementation it must contain a single PersonaFacet.
    pub profile: Profile,

    /// Hierarchical, json-like data structure, encoded using multicodec library,
    /// encrypted with the persona's keys, and stored on the home server
    pub priv_data: Vec<u8>, // TODO maybe multicodec output?
}

impl OwnProfile {
    pub fn new(profile: &Profile, private_data: &[u8]) -> Self {
        Self { profile: profile.clone(), priv_data: private_data.to_owned() }
    }
}

// NOTE the binary blob to be signed is rust-specific: Strings are serialized to a u64 (size) and the encoded string itself.
// TODO consider if this is platform-agnostic enough, especially when combined with capnproto
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct RelationSignablePart {
    pub relation_type: String,
    pub signer_id: ProfileId,
    pub peer_id: ProfileId,
    // TODO is a nonce needed?
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationHalfProof {
    pub relation_type: String,
    pub signer_id: ProfileId,
    pub peer_id: ProfileId,
    pub signature: Signature,
    // TODO is a nonce needed?
}

impl RelationHalfProof {
    pub fn new(relation_type: &str, peer_id: &ProfileId, signer: &Signer) -> Self {
        let signable = RelationSignablePart::new(relation_type, &signer.profile_id(), peer_id);
        Self {
            relation_type: relation_type.to_owned(),
            signer_id: signer.profile_id().to_owned(),
            peer_id: peer_id.to_owned(),
            signature: signable.sign(signer),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RelationProof {
    pub relation_type: String, // TODO inline halfproof fields with macro, if possible at all
    pub a_id: ProfileId,
    pub a_signature: Signature,
    pub b_id: ProfileId,
    pub b_signature: Signature,
    // TODO is a nonce needed?
}

// TODO should we ignore this in early stages?
/// This invitation allows a persona to register on the specified home.
//#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
//pub struct HomeInvitation {
//    pub home_id: ProfileId,
//
//    /// A unique string that identifies the invitation
//    pub voucher: String,
//
//    /// The signature of the home
//    pub signature: Signature,
//    // TODO is a nonce needed?
//    // TODO is an expiration time needed?
//}
//
//impl HomeInvitation {
//    pub fn new(home_id: &ProfileId, voucher: &str, signature: &Signature) -> Self {
//        Self {
//            home_id: home_id.to_owned(),
//            voucher: voucher.to_owned(),
//            signature: signature.to_owned(),
//        }
//    }
//}

//impl<'a> From<&'a [u8]> for ProfileId {
//    fn from(src: &'a [u8]) -> Self {
//        ProfileId(src.to_owned())
//    }
//}
//
//impl<'a> From<&'a ProfileId> for &'a [u8] {
//    fn from(src: &'a ProfileId) -> Self {
//        &src.0
//    }
//}
//
//impl<'a> From<ProfileId> for Vec<u8> {
//    fn from(src: ProfileId) -> Self {
//        src.0
//    }
//}
//
//impl<'a> TryFrom<&'a str> for ProfileId {
//    type Error = multibase::Error;
//    fn try_from(src: &'a str) -> Result<Self, Self::Error> {
//        let (_base, binary) = multibase::decode(src)?;
//        Ok(ProfileId(binary))
//    }
//}
//
//impl<'a> From<&'a ProfileId> for String {
//    fn from(src: &'a ProfileId) -> Self {
//        multibase::encode(multibase::Base::Base64url, &src.0)
//    }
//}
//
//impl<'a> From<ProfileId> for String {
//    fn from(src: ProfileId) -> Self {
//        Self::from(&src)
//    }
//}
//
//impl std::fmt::Display for ProfileId {
//    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//        write!(f, "{}", String::from(self))
//    }
//}
//
//impl<'a> From<&'a PublicKey> for String {
//    fn from(src: &'a PublicKey) -> Self {
//        multibase::encode(multibase::Base::Base64url, &src.0)
//    }
//}
//
//impl<'a> From<PublicKey> for String {
//    fn from(src: PublicKey) -> Self {
//        Self::from(&src)
//    }
//}
//
//impl std::fmt::Display for PublicKey {
//    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//        write!(f, "{}", String::from(self))
//    }
//}

fn serialize_multiaddr_vec<S>(x: &Vec<Multiaddr>, s: S) -> std::result::Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut seq = s.serialize_seq(Some(x.len()))?;
    for mr in x {
        match seq.serialize_element(&mr.to_string()) {
            Ok(_) => {
                ();
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
    seq.end()
}

fn deserialize_multiaddr_vec<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<Multiaddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let mapped: Vec<String> = DeSer::deserialize(deserializer)?;
    let mut res = Vec::new();
    for str_ma in mapped.iter() {
        match str_ma.to_multiaddr() {
            Ok(multi) => {
                res.push(multi);
            }
            Err(e) => {
                return Err(D::Error::custom(e));
            }
        }
    }
    Ok(res)
}

impl Profile {
    pub fn new(id: &ProfileId, public_key: &PublicKey, facet: &ProfileFacet) -> Self {
        Self { id: id.to_owned(), public_key: public_key.to_owned(), facet: facet.to_owned() }
    }

    pub fn new_home(id: ProfileId, public_key: PublicKey, address: Multiaddr) -> Self {
        let facet = HomeFacet { addrs: vec![address], data: vec![] };

        Self { id, public_key, facet: ProfileFacet::Home(facet) }
    }
}

impl RelationSignablePart {
    pub(crate) fn new(relation_type: &str, signer_id: &ProfileId, peer_id: &ProfileId) -> Self {
        Self {
            relation_type: relation_type.to_owned(),
            signer_id: signer_id.to_owned(),
            peer_id: peer_id.to_owned(),
        }
    }

    pub(crate) fn serialized(&self) -> Vec<u8> {
        // TODO unwrap() can fail here in some special cases: when there is a limit set and it's exceeded - or when .len() is
        //      not supported for the types to be serialized. Neither is possible here, so the unwrap will not fail.
        //      But still, to be on the safe side, this serialization shoule be swapped later with a call that cannot fail.
        // TODO consider using unwrap_or( Vec::new() ) instead
        serialize(self).unwrap()
    }

    fn sign(&self, signer: &Signer) -> Signature {
        signer.sign(&self.serialized())
    }
}

impl<'a> From<&'a RelationHalfProof> for RelationSignablePart {
    fn from(src: &'a RelationHalfProof) -> Self {
        RelationSignablePart {
            relation_type: src.relation_type.clone(),
            signer_id: src.signer_id.clone(),
            peer_id: src.peer_id.clone(),
        }
    }
}

impl RelationProof {
    pub const RELATION_TYPE_HOSTED_ON_HOME: &'static str = "hosted_on_home";
    pub const RELATION_TYPE_ENABLE_CALLS_BETWEEN: &'static str = "enable_call_between";

    pub fn new(
        relation_type: &str,
        a_id: &ProfileId,
        a_signature: &Signature,
        b_id: &ProfileId,
        b_signature: &Signature,
    ) -> Self {
        if a_id < b_id {
            Self {
                relation_type: relation_type.to_owned(),
                a_id: a_id.to_owned(),
                a_signature: a_signature.to_owned(),
                b_id: b_id.to_owned(),
                b_signature: b_signature.to_owned(),
            }
        }
        // TODO decide on inverting relation_type if needed, e.g. `a_is_home_of_b` vs `b_is_home_of_a`
        else {
            Self {
                relation_type: relation_type.to_owned(),
                a_id: b_id.to_owned(),
                a_signature: b_signature.to_owned(),
                b_id: a_id.to_owned(),
                b_signature: a_signature.to_owned(),
            }
        }
    }

    pub fn sign_remaining_half(
        half_proof: &RelationHalfProof,
        signer: &Signer,
    ) -> Result<Self, Error> {
        let my_profile_id = signer.profile_id().to_owned();
        if half_proof.peer_id != my_profile_id {
            Err(ErrorKind::RelationSigningFailed)?
        }

        let signable = RelationSignablePart::new(
            &half_proof.relation_type,
            &my_profile_id,
            &half_proof.signer_id,
        );
        Ok(Self::new(
            &half_proof.relation_type,
            &half_proof.signer_id,
            &half_proof.signature,
            &my_profile_id,
            &signable.sign(signer),
        ))
    }

    // TODO relation-type should be more sophisticated once we have a proper metainfo schema there
    pub fn accessible_by(&self, app: &ApplicationId) -> bool {
        self.relation_type == app.0
    }

    pub fn peer_id(&self, my_id: &ProfileId) -> Result<&ProfileId, Error> {
        if self.a_id == *my_id {
            return Ok(&self.b_id);
        }
        if self.b_id == *my_id {
            return Ok(&self.a_id);
        }
        Err(ErrorKind::PeerIdRetreivalFailed)?
    }

    pub fn peer_signature(&self, my_id: &ProfileId) -> Result<&Signature, Error> {
        if self.a_id == *my_id {
            return Ok(&self.b_signature);
        }
        if self.b_id == *my_id {
            return Ok(&self.a_signature);
        }
        Err(ErrorKind::PeerIdRetreivalFailed)?
    }
}
