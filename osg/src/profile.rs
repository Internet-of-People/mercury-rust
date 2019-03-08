use std::cell::RefCell;
use std::rc::Rc;

use failure::Fallible;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

pub type ProfilePtr = Rc<RefCell<Profile>>;

// TODO should all operations below be async?
pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Option<ProfilePtr>;
    fn create(&mut self, id: &ProfileId) -> Fallible<ProfilePtr>;
    // clear up links and attributes to leave an empty tombstone in place of the profile.
    fn remove(&mut self, id: &ProfileId) -> Fallible<()>;

    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;

    // TODO should these be located here or in the vault instead?
    // fn publish(&mut self) -> Fallible<()>;
    // fn restore(&mut self) -> Fallible<()>;
}

pub trait Profile {
    fn id(&self) -> ProfileId;
    fn attributes(&self) -> Fallible<AttributeMap>;
    fn links(&self) -> Fallible<Vec<Link>>;

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link>;
    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()>;

    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()>;
    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()>;

    //fn sign(&self, data: &[u8]) -> Signature;
    //fn get_signer(&self) -> Arc<Signer>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LocalProfile {
    profile_data: ProfileData,
    remote_version: Option<u32>,
    modified: bool,
}

impl LocalProfile {
    pub fn new(id: &ProfileId) -> Self {
        Self {
            profile_data: ProfileData::default(id),
            remote_version: None, // TODO fill this after trying discovery from remote storage
            modified: false,
        }
    }

    pub fn from(profile: ProfileData) -> Self {
        Self {
            profile_data: profile,
            remote_version: None, // TODO fill this after trying discovery from remote storage
            modified: true,       // TODO what should we do here?
        }
    }
}

impl Profile for LocalProfile {
    fn id(&self) -> ProfileId {
        self.profile_data.id.clone()
    }

    fn attributes(&self) -> Fallible<AttributeMap> {
        Ok(self.profile_data.attributes.clone())
    }

    fn links(&self) -> Fallible<Vec<Link>> {
        Ok(self.profile_data.links.clone())
    }

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link> {
        let link = Link {
            peer_profile: peer_profile.to_owned(),
        };
        self.profile_data.links.push(link.clone());
        Ok(link)
    }

    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()> {
        self.profile_data
            .links
            .retain(|link| link.peer_profile != *peer_profile);
        Ok(())
    }

    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()> {
        self.profile_data
            .attributes
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }

    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()> {
        self.profile_data.attributes.remove(key);
        Ok(())
    }
}
