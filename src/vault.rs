use std::collections::HashMap;

use crate::types::*;



pub struct Link
{
    pub id: LinkId,
//    pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    pub peer_profile: ProfileId,
    pub metadata: HashMap<AttributeId,AttributeValue>,
}


pub struct Profile
{
    pub id: ProfileId,
    pub links: Vec<Link>,
    pub metadata: HashMap<AttributeId,AttributeValue>,
}


// TODO should all operations below be async?
pub trait ProfileData // NOTE this should be impl Profile, but it would need implementation immediately to compile
{
    fn create_link(peer_profile: &ProfileId) -> Link;
    fn remove_link(id: &LinkId);

    fn set_attribute(key: AttributeId, value: AttributeValue);
    fn clear_attribute(key: &AttributeId);

    fn list_followers() -> Vec<Link>;
}


pub trait ProfileVault
{
    fn list_profiles() -> Vec<ProfileId>;
    fn get_profile(id: &ProfileId) -> Profile; // TODO or should list_profiles() return Vec<Profile> and drop this function?
    fn create_profile() -> Profile;
    // TODO what does this mean? Purge related metadata from local storage?
    // fn remove_profile(id: &ProfileId);

    fn get_active_profile() -> Option<ProfileId>;
    fn set_active_profile(id: &ProfileId);
}
