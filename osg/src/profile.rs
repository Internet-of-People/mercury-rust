use std::cell::RefCell;
use std::rc::Rc;

use failure::Fallible;

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
