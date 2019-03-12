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

    pub fn empty(id: &ProfileId) -> Self {
        Self {
            id: id.to_owned(),
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    pub fn links(&self) -> &Vec<Link> {
        &self.links
    }

    pub fn create_link(&mut self, with_id: &ProfileId) {
        // TODO check duplicates here
        self.links.push(Link {
            peer_profile: with_id.to_owned(),
        })
    }

    pub fn remove_link(&mut self, with_id: &ProfileId) {
        self.links.retain(|link| link.peer_profile != *with_id)
    }

    pub fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    pub fn set_attribute(&mut self, key: AttributeId, value: AttributeValue) {
        self.attributes.insert(key, value);
    }

    pub fn clear_attribute(&mut self, key: &AttributeId) {
        self.attributes.remove(key);
    }
}

// TODO remove this after TryFrom has been stabilized
pub trait TryFrom<T>: Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}
