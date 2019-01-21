use std::collections::HashMap;
use std::rc::Rc;

use crate::types::*;



pub struct Link
{
    pub id: LinkId,
//    pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    pub peer_profile: ProfileId,
//    pub metadata: HashMap<AttributeId,AttributeValue>,
}



pub trait Signer
{
    fn profile_id(&self) -> &ProfileId;
    fn public_key(&self) -> &PublicKey;
    fn sign(&self, data: &[u8]) -> Signature;
    //fn encrypt(&self, data: &[u8], target: &PublicKey) -> Vec<u8>;
}


pub struct KeyVault {}
impl KeyVault
{
    pub fn list(&self) -> Vec<Rc<Signer>> { unimplemented!() }
    pub fn get(&self, _profile_id: &ProfileId) -> Rc<Signer> { unimplemented!() }
    pub fn create(&mut self) -> Rc<Signer> { unimplemented!() }

    pub fn get_active(&self) -> Option<ProfileId> { unimplemented!() }
    pub fn set_active(&mut self, _id: &ProfileId) { unimplemented!() }
}



//pub trait ProfileVault
//{
//    fn list_profiles() -> Vec<ProfileId>;
//    fn get_profile(id: &ProfileId) -> Box<Profile>; // TODO or should list_profiles() return Vec<Profile> and drop this function?
//    fn create_profile() -> Box<Profile>;
//    // TODO what does this mean? Purge related metadata from local storage?
//    // fn remove_profile(id: &ProfileId);
//
//    fn get_active_profile() -> Option<ProfileId>;
//    fn set_active_profile(id: &ProfileId);
//}
