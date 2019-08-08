use std::cell::RefCell;
use std::rc::Rc;

use failure::Fallible;

use claims::model::*;

pub type ProfilePtr = Rc<RefCell<Profile>>;

// TODO these operations are basically the same as of struct PublicProfileData
//      but can fail. We should somehow merge them together if this storage impl is kept.
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

    fn to_data(&self) -> Fallible<PrivateProfileData> {
        // TODO fill in claims here
        Ok(PrivateProfileData::without_morpheus_claims(
            PublicProfileData::new(
                self.public_key()?,
                self.version()?,
                self.links()?,
                self.attributes()?,
            ),
            self.private_data()?,
        ))
    }
}
