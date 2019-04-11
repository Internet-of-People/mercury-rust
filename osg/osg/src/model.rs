use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

pub type ProfileId = keyvault::multicipher::MKeyId;
pub type Version = u64; // monotonically increasing, e.g. normal version, unix datetime or blockheight
pub type AttributeId = String;
pub type AttributeValue = String;
pub type AttributeMap = HashMap<AttributeId, AttributeValue>;

// TODO generalize links (i.e. edges) between two profiles into verifiable claims,
//      i.e. signed hyperedges in the graph with any number of referenced profiles
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileData {
    id: ProfileId,
    version: Version,
    links: Vec<Link>,
    attributes: AttributeMap,
    // TODO consider adding both a public key and a signature of the profile data here
}

impl ProfileData {
    pub fn create(
        id: ProfileId,
        version: Version,
        links: Vec<Link>,
        attributes: AttributeMap,
    ) -> Self {
        Self { id, version, links, attributes }
    }

    pub fn new(id: &ProfileId) -> Self {
        Self {
            id: id.to_owned(),
            version: 1,
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    pub fn tombstone(id: &ProfileId, last_version: Version) -> Self {
        Self {
            id: id.to_owned(),
            version: last_version + 1,
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    // TODO these operations are basically the same as of trait Profile
    //      (created towards the concepts of RpcProfile) but cannot fail.
    //      We should either kill trait Profile or fit it like this.
    pub fn id(&self) -> &ProfileId {
        &self.id
    }
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn increase_version(&mut self) {
        self.version += 1;
    }
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    pub fn links(&self) -> &Vec<Link> {
        &self.links
    }

    pub fn create_link(&mut self, with_id: &ProfileId) -> Link {
        let link = Link { peer_profile: with_id.to_owned() };
        if !self.links.contains(&link) {
            self.links.push(link.clone());
        }
        link
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
