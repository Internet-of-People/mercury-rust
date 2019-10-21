use std::convert::{TryFrom, TryInto};
use std::str::FromStr;

use failure::{err_msg, Fallible};
use multiaddr::Multiaddr;
use serde::{Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};

use crate::home::discovery::KnownHomeNode;
pub use claims::claim_schema::{ClaimSchemas, SchemaId, SchemaVersion};
use claims::model::*;
use did::vault::*;
use mercury_home_protocol::primitives::{deserialize_multiaddr_vec, serialize_multiaddr_vec};

pub type MessageContent = Vec<u8>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Message {
    pub message: MessageContent,
    pub sender: ProfileId,
    pub receiver: ProfileId,
    pub timestamp: TimeStamp,
}

pub type DataUri = String;
pub type ImageFormat = String;
pub type ImageBlob = Vec<u8>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Image {
    pub format: ImageFormat,
    pub blob: ImageBlob,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct VaultEntry {
    pub id: String,
    pub label: String,
    #[serde(serialize_with = "serialize_avatar", deserialize_with = "deserialize_avatar")]
    pub avatar: Image,
    pub state: String,
}

impl TryFrom<&ProfileVaultRecord> for VaultEntry {
    type Error = failure::Error;
    fn try_from(src: &ProfileVaultRecord) -> Fallible<Self> {
        let metadata: PersonaCustomData = src.metadata().as_str().try_into()?;
        Ok(VaultEntry {
            id: src.id().to_string(),
            label: src.label(),
            avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
            state: "TODO".to_owned(), // TODO this will probably need another query to context
        })
    }
}

impl TryInto<ProfileVaultRecord> for &VaultEntry {
    type Error = failure::Error;
    fn try_into(self) -> Result<ProfileVaultRecord, Self::Error> {
        Ok(ProfileVaultRecord::new(
            ProfileId::from_str(&self.id)?,
            self.label.to_owned(),
            Default::default(), // TODO fill in metadata properly
        ))
    }
}

// TODO serialize stored image format with the blob, do not hardwire 'png'
pub fn serialize_avatar<S: Serializer>(avatar: &Image, serializer: S) -> Result<S::Ok, S::Error> {
    // About the format used here, see https://en.wikipedia.org/wiki/Data_URI_scheme
    let data_uri = format!("data:image/{};base64,{}", avatar.format, base64::encode(&avatar.blob));
    serializer.serialize_str(&data_uri)
}

pub fn deserialize_avatar<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Image, D::Error> {
    use serde::{de, Deserialize};
    let data_uri = String::deserialize(deserializer)?;

    // TODO do we need support for more encodings than just base64 here?
    let re = regex::Regex::new(r"(?x)data:image/(?P<format>\w+);base64,(?P<data>.*)")
        .map_err(de::Error::custom)?;
    let captures = re
        .captures(&data_uri)
        .ok_or_else(|| de::Error::custom("Provided image is not in DataURI format"))?;
    let (format, encoded_avatar) = (captures["format"].to_owned(), &captures["data"]);
    let blob = base64::decode(encoded_avatar).map_err(de::Error::custom)?;
    Ok(Image { format, blob })
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PersonaCustomData {
    pub image_blob: ImageBlob,
    pub image_format: ImageFormat,
}

impl TryFrom<&[u8]> for PersonaCustomData {
    type Error = failure::Error;
    fn try_from(src: &[u8]) -> Fallible<Self> {
        Ok(serde_json::from_slice(src)?)
    }
}

impl TryInto<Vec<u8>> for PersonaCustomData {
    type Error = failure::Error;
    fn try_into(self) -> Fallible<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }
}

impl TryFrom<&str> for PersonaCustomData {
    type Error = failure::Error;
    fn try_from(src: &str) -> Fallible<Self> {
        Ok(serde_json::from_str(src)?)
    }
}

impl TryInto<String> for PersonaCustomData {
    type Error = failure::Error;
    fn try_into(self) -> Fallible<String> {
        Ok(serde_json::to_string(&self)?)
    }
}

// TODO do we need support for more encodings than just base64 here?
pub fn parse_avatar(data_uri: &str) -> Fallible<(ImageFormat, ImageBlob)> {
    let re = regex::Regex::new(r"(?x)data:image/(?P<format>\w+);base64,(?P<data>.*)")?;
    let captures = re
        .captures(&data_uri)
        .ok_or_else(|| err_msg("Provided image is not in DataURI format, see https://en.wikipedia.org/wiki/Data_URI_scheme"))?;
    let (format, encoded_avatar) = (&captures["format"], &captures["data"]);
    let avatar = base64::decode(encoded_avatar)?;
    Ok((format.to_owned(), avatar))
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AttributePath {
    pub did: String,
    pub attribute_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiClaim {
    id: ContentId,
    subject_id: String,
    subject_label: String,
    schema_id: String,
    schema_name: String,
    content: serde_json::Value,
    proof: Vec<ApiClaimProof>,
}

impl ApiClaim {
    pub fn try_from(
        src: &Claim,
        subject_label: ProfileLabel,
        schema_registry: &dyn ClaimSchemas,
    ) -> Fallible<Self> {
        let signable = src.signable_part();
        let schema_id = signable.typed_content.schema_id().to_owned();
        let schema_name = schema_registry.get(&schema_id)?.name().to_owned();
        Ok(Self {
            id: src.id(),
            subject_id: signable.subject_id.to_string(),
            subject_label,
            schema_id,
            schema_name,
            content: signable.typed_content.content().to_owned(),
            proof: src.proofs().iter().map(|proof| proof.into()).collect(),
        })
    }
}

impl TryInto<Claim> for &ApiClaim {
    type Error = failure::Error;

    fn try_into(self) -> Result<Claim, Self::Error> {
        let subject_id = self.subject_id.parse()?;
        let proof = self
            .proof
            .iter()
            .filter_map(|proof| {
                // TODO log conversion errors
                proof.try_into().ok()
            })
            .collect();
        Ok(Claim::new(subject_id, self.schema_id.to_owned(), self.content.to_owned(), proof))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiClaimProof {
    pub signer_id: String,
    pub signed_message: ApiSignedMessage,
    pub issued_at: TimeStamp,
    pub valid_until: TimeStamp,
}

impl From<&ClaimProof> for ApiClaimProof {
    fn from(src: &ClaimProof) -> Self {
        ApiClaimProof {
            signer_id: src.signer_id().to_string(),
            signed_message: src.signed_message().into(),
            issued_at: src.issued_at(),
            valid_until: src.valid_until(),
        }
    }
}

impl TryFrom<&ApiClaimProof> for ClaimProof {
    type Error = failure::Error;

    fn try_from(src: &ApiClaimProof) -> Result<Self, Self::Error> {
        Ok(ClaimProof::new(
            src.signer_id.parse()?,
            (&src.signed_message).try_into()?,
            src.issued_at.to_owned(),
            src.valid_until.to_owned(),
        ))
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiSignedMessage {
    public_key: String,
    message: String,
    signature: String,
}

impl From<&SignedMessage> for ApiSignedMessage {
    fn from(src: &SignedMessage) -> Self {
        ApiSignedMessage {
            public_key: src.public_key().to_string(),
            message: multibase::encode(multibase::Base64url, src.message()),
            signature: src.signature().to_string(),
        }
    }
}

impl TryFrom<&ApiSignedMessage> for SignedMessage {
    type Error = failure::Error;

    fn try_from(src: &ApiSignedMessage) -> Result<Self, Self::Error> {
        let (_base, message) = multibase::decode(&src.message)?;
        Ok(SignedMessage::new(src.public_key.parse()?, message, src.signature.parse()?))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ClaimPath {
    pub did: String,
    pub claim_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClaimSchema {
    id: String,
    label: String,
    author: String,
    version: u32,
    content: serde_json::Value,
    ordering: Vec<String>,
}

impl ClaimSchema {
    fn new(
        id: impl ToString,
        label: impl ToString,
        content: serde_json::Value,
        ordering: Vec<String>,
    ) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            content,
            ordering,
            // TODO these should be received and filled from request values
            author: Default::default(),
            version: Default::default(),
        }
    }
}

impl From<&SchemaVersion> for ClaimSchema {
    fn from(model: &SchemaVersion) -> Self {
        Self::new(model.id(), model.name(), model.content().clone(), model.ordering().to_vec())
    }
}

impl Into<SchemaVersion> for ClaimSchema {
    fn into(self) -> SchemaVersion {
        SchemaVersion::new_with_order(
            self.id,
            self.author,
            self.label,
            self.version,
            self.content,
            self.ordering,
        )
    }
}

//impl Into<SchemaVersion> for &ClaimSchema {
//    fn into(self) -> SchemaVersion {
//        SchemaVersion::new_with_order(
//            &self.id,
//            &self.author,
//            &self.label,
//            self.version,
//            self.content.clone(),
//            self.ordering.clone(),
//        )
//    }
//}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CreateClaim {
    pub schema: SchemaId, // TODO multihash?
    pub content: serde_json::Value,
}

impl TryFrom<Claim> for CreateClaim {
    type Error = failure::Error;

    fn try_from(src: Claim) -> Result<Self, Self::Error> {
        let signable = src.signable_part();
        Ok(CreateClaim {
            schema: signable.typed_content.schema_id().to_owned(),
            content: signable.typed_content.content().to_owned(),
        })
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct HomeNode {
    pub home_did: String,
    pub latency_ms: u32,
    #[serde(serialize_with = "serialize_multiaddr_vec")]
    #[serde(deserialize_with = "deserialize_multiaddr_vec")]
    pub underlay_addrs: Vec<Multiaddr>,
    pub public: serde_json::Value,
}

impl From<&KnownHomeNode> for HomeNode {
    fn from(n: &KnownHomeNode) -> Self {
        Self {
            home_did: n.profile.id().to_string(),
            latency_ms: n.latency.map(|d| d.as_millis() as u32).unwrap_or(u32::max_value()),
            underlay_addrs: n.addrs(),
            public: serde_json::Value::Null,
        }
    }
}
