use failure::Fallible;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
use crate::profile::*;

// TODO should all operations below be async?
pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfilePtr>;
    fn set(&mut self, id: &ProfileId, profile: ProfilePtr) -> Fallible<()>;
    // clear up links and attributes to leave an empty tombstone in place of the profile.
    fn clear(&mut self, id: &ProfileId) -> Fallible<()>;

    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;

    // TODO should these be located here or in the vault instead?
    // fn publish(&mut self) -> Fallible<()>;
    // fn restore(&mut self) -> Fallible<()>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalProfileRepository {
    pub profiles: Vec<ProfileData>,
}

impl LocalProfileRepository {}

// TODO implement keeping serialized profiles in profiles.dat (near vault.dat)
impl ProfileRepository for LocalProfileRepository {
    fn get(&self, _id: &ProfileId) -> Fallible<ProfilePtr> {
        unimplemented!()
    }
    fn set(&mut self, _id: &ProfileId, _profile: ProfilePtr) -> Fallible<()> {
        unimplemented!()
    }
    fn clear(&mut self, _id: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }
    fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!()
    }
}
