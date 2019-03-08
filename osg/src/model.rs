use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

pub type AttributeId = String;
pub type AttributeValue = String;
pub type Signature = Vec<u8>;
pub type PublicKey = Vec<u8>;
pub type ProfileId = keyvault::multicipher::MKeyId;

pub type AttributeMap = HashMap<AttributeId, AttributeValue>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileData {
    pub id: ProfileId,
    pub links: Vec<Link>,
    pub attributes: AttributeMap,
}

impl ProfileData {
    pub fn new(id: ProfileId, links: Vec<Link>, attributes: AttributeMap) -> Self {
        Self {
            id,
            links,
            attributes,
        }
    }

    pub fn default(id: &ProfileId) -> Self {
        Self {
            id: id.to_owned(),
            links: Default::default(),
            attributes: Default::default(),
        }
    }
}
