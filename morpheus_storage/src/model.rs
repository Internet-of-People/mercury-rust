use failure::ensure;
use serde::{de::Visitor, Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};

//pub type ProfileId = Vec<u8>;
pub type AttributeId = String;
pub type AttributeValue = String; // StructOpt needs FromStr, MessagePack needs Vec<u8>
pub type Signature = Vec<u8>;
pub type PublicKey = Vec<u8>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct ProfileId {
    #[serde(serialize_with = "serialize_byte_vec")]
    #[serde(deserialize_with = "deserialize_byte_vec")]
    pub id: Vec<u8>,
}

impl<'a> From<&'a ProfileId> for String {
    fn from(src: &'a ProfileId) -> Self {
        let mut output = multibase::encode(multibase::Base58btc, &src.id);
        output.insert(0, 'I');
        output
    }
}

impl<'a> From<ProfileId> for String {
    fn from(src: ProfileId) -> Self {
        Self::from(&src)
    }
}

impl std::fmt::Display for ProfileId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl std::str::FromStr for ProfileId {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut chars = src.chars();
        ensure!(
            chars.next() == Some('I'),
            "Profile identifier must start with 'I'"
        );
        let (_base, binary) = multibase::decode(chars.as_str())?;
        Ok(ProfileId { id: binary })
    }
}

//pub type LinkId = ProfileId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}

pub(crate) fn serialize_byte_vec<S>(data: &[u8], ser: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    ser.serialize_bytes(data)
}

pub(crate) fn deserialize_byte_vec<'de, D>(deser: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    deser.deserialize_bytes(BytesVisitor {})
}

struct BytesVisitor;

impl<'de> Visitor<'de> for BytesVisitor {
    type Value = Vec<u8>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("[bytes]")
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        Ok(v.to_owned())
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        Ok(v)
    }
}

//pub(crate) fn serialize_profile_id<S>(data: &ProfileId, ser: S) -> Result<S::Ok, S::Error> where S: Serializer,
//    { ser.serialize_bytes(&data.id) }
//
//pub(crate) fn deserialize_profile_id<'de, D>(deser: D) -> Result<ProfileId, D::Error> where D: Deserializer<'de>,
//    { deser.deserialize_bytes( BytesVisitor{} ).map( |bytes| ProfileId{id: bytes} ) }
