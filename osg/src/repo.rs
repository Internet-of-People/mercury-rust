use std::collections::HashMap;

use failure::{err_msg, Fallible};
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
//use crate::profile::*;

// TODO should all operations below be async?
pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData>;
    fn set(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()>;
    // clear up links and attributes to leave an empty tombstone in place of the profile.
    fn clear(&mut self, id: &ProfileId) -> Fallible<()>;

    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;

    // TODO should these be located here or in the vault instead?
    // fn publish(&mut self) -> Fallible<()>;
    // fn restore(&mut self) -> Fallible<()>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalProfileRepository {
    pub profiles: HashMap<ProfileId, ProfileData>,
}

impl LocalProfileRepository {}

// TODO implement keeping serialized profiles in profiles.dat (near vault.dat)
impl ProfileRepository for LocalProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData> {
        // TODO we probably should also have some nicely typed errors here
        self.profiles
            .get(id)
            .map(|prof_ref| prof_ref.to_owned())
            .ok_or_else(|| err_msg("Profile not found"))
    }
    fn set(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()> {
        self.profiles.insert(id, profile);
        Ok(())
    }
    fn clear(&mut self, id: &ProfileId) -> Fallible<()> {
        self.profiles.remove(id);
        Ok(())
    }
    fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        // TODO how to implement this?
        unimplemented!()
    }
}
