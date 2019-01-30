//use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use morpheus_storage::*;
//use crate::types::{Link, PublicKey, Signature};



//pub trait Signer
//{
//    fn profile_id(&self) -> &ProfileId;
//    fn public_key(&self) -> &PublicKey;
//    fn sign(&self, data: &[u8]) -> Signature;
//    //fn encrypt(&self, data: &[u8], target: &PublicKey) -> Vec<u8>;
//}
//
//
//pub struct KeyVault {}
//impl<K: KeyDerivationCrypto> KeyVault
//{
//    pub fn list(&self) -> Vec<ProfileId> { unimplemented!() }
//    pub fn get_public(&self, _profile_id: &ProfileId) -> PublicKey { unimplemented!() }
//    pub fn get(&self, _profile_id: &ProfileId, authentication: TODO) -> Rc<Signer> { unimplemented!() }
//    pub fn create(&mut self) -> Rc<Signer> { unimplemented!() }
//}



// TODO several functions should return Result<> instead, but that needs building our own proper error data structures
pub struct ProfileVault;
impl ProfileVault
{
    pub fn list(&self) -> Vec<ProfileId> { unimplemented!() } // TODO should this return an iterator instead?
    pub fn get(&self, id: &ProfileId) -> Option< Arc<RwLock<Profile>> > { unimplemented!() } // TODO or should list_profiles() return Vec<Profile> and drop this function?
    pub fn create(&self) -> Arc<RwLock<Profile>> { unimplemented!() }
    // TODO what does this mean? Purge related metadata from local storage plus don't show it in the list,
    //      or maybe also delete all links/follows with other profiles
    fn remove(&self, id: &ProfileId) { unimplemented!() }

    pub fn get_active(&self) -> Option<ProfileId> { unimplemented!() }
    pub fn set_active(&self, id: &ProfileId) -> bool { unimplemented!() }
}
