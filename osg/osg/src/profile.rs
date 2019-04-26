use std::cell::RefCell;
use std::convert::TryFrom;
use std::rc::Rc;

use failure::Fallible;
//use serde_derive::{Deserialize, Serialize};

use crate::model::*;

pub type ProfilePtr = Rc<RefCell<Profile>>;

pub trait Profile {
    fn id(&self) -> ProfileId;
    fn public_key(&self) -> Fallible<PublicKey>;
    fn version(&self) -> Fallible<Version>;
    fn attributes(&self) -> Fallible<AttributeMap>;
    fn links(&self) -> Fallible<Vec<Link>>;
    fn private_data(&self) -> Fallible<Vec<u8>>;

    fn set_version(&mut self, version: Version) -> Fallible<()>;

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link>;
    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()>;

    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()>;
    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()>;

    //fn sign(&self, data: &[u8]) -> Signature;
    //fn get_signer(&self) -> Arc<Signer>;
}

impl TryFrom<ProfilePtr> for PrivateProfileData {
    type Error = failure::Error;
    fn try_from(value: ProfilePtr) -> Result<Self, Self::Error> {
        let profile = value.borrow();
        Ok(PrivateProfileData::create(
            PublicProfileData::create(
                profile.public_key()?,
                profile.version()?,
                profile.links()?,
                profile.attributes()?,
            ),
            profile.private_data()?,
        ))
    }
}
