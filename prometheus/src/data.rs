use std::convert::{TryFrom, TryInto};

use failure::{err_msg, Fallible};
use serde::{Deserializer, Serializer};
use serde_derive::{Deserialize, Serialize};

use claims::{api::*, model::*};
use did::vault::*;

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
        let metadata: ProfileMetadata = src.metadata().as_str().try_into()?;
        Ok(VaultEntry {
            id: src.id().to_string(),
            label: src.label(),
            avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
            state: "TODO".to_owned(), // TODO this will probably need another query to context
        })
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
pub struct ProfileMetadata {
    pub image_blob: ImageBlob,
    pub image_format: ImageFormat,
}

impl TryFrom<&[u8]> for ProfileMetadata {
    type Error = failure::Error;
    fn try_from(src: &[u8]) -> Fallible<Self> {
        Ok(serde_json::from_slice(src)?)
    }
}

impl TryInto<Vec<u8>> for ProfileMetadata {
    type Error = failure::Error;
    fn try_into(self) -> Fallible<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }
}

impl TryFrom<&str> for ProfileMetadata {
    type Error = failure::Error;
    fn try_from(src: &str) -> Fallible<Self> {
        Ok(serde_json::from_str(src)?)
    }
}

impl TryInto<String> for ProfileMetadata {
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

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ApiClaim {
    id: ContentId,
    subject_id: String,
    subject_label: String,
    schema_id: String,
    schema_name: String,
    content: serde_json::Value,
    proof: Vec<ClaimProof>,
    presentation: Vec<ClaimPresentation>,
}

impl ApiClaim {
    pub fn try_from(
        src: &Claim,
        subject_label: ProfileLabel,
        schema_registry: &ClaimSchemaRegistry,
    ) -> Fallible<Self> {
        let schema_name = schema_registry.get(&src.schema)?.name().to_owned();
        Ok(Self {
            id: src.id(),
            subject_id: src.subject_id.to_string(),
            subject_label,
            schema_id: src.schema.to_owned(),
            schema_name,
            content: serde_json::from_slice(&src.content)?,
            proof: src.proof.to_owned(),
            presentation: src.presentation.to_owned(),
        })
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
        Self { id: id.to_string(), label: label.to_string(), content, ordering }
    }
}

impl From<&claims::claim_schema::SchemaVersion> for ClaimSchema {
    fn from(model: &claims::claim_schema::SchemaVersion) -> Self {
        Self::new(model.id(), model.name(), model.content().clone(), model.ordering().to_vec())
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CreateClaim {
    pub schema: ContentId, // TODO multihash?
    pub content: serde_json::Value,
}
