use serde_derive::{Deserialize, Serialize};

pub type AttributeId = String;
pub type AttributeValue = String;
pub type Signature = Vec<u8>;
pub type PublicKey = Vec<u8>;
pub type ProfileId = morpheus_keyvault::multicipher::MKeyId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}
