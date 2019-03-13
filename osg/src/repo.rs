use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use failure::{err_msg, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

// TODO should all operations below be async?
pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData>;
    fn set(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()>;
    // clear up links and attributes to leave an empty tombstone in place of the profile.
    fn clear(&mut self, id: &ProfileId) -> Fallible<()>;

    // TODO this shouldn't be here, an external clawler/explorer service should be used
    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LocalProfileRepository {
    profiles: HashMap<ProfileId, ProfileData>,
    #[serde(skip)]
    filename: PathBuf,
}

impl LocalProfileRepository {
    pub fn create(filename: &PathBuf) -> Fallible<Self> {
        let this = Self {
            profiles: Default::default(),
            filename: filename.to_owned(),
        };
        this.save()?;
        Ok(this)
    }

    fn save(&self) -> Fallible<()> {
        debug!("Saving profile repository to {:?}", self.filename);
        if let Some(repo_dir) = self.filename.parent() {
            debug!("Recursively Creating directory {:?}", repo_dir);
            std::fs::create_dir_all(repo_dir)?;
        }

        let repo_file = File::create(&self.filename)?;
        bincode::serialize_into(repo_file, self)?;
        Ok(())
    }

    pub fn load(filename: &PathBuf) -> Fallible<Self> {
        debug!("Loading profile repository from {:?}", filename);
        let repo_file = File::open(filename)?;
        let mut repo: Self = bincode::deserialize_from(repo_file)?;
        repo.filename = filename.to_owned();
        Ok(repo)
    }
}

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
        self.save()?;
        Ok(())
    }

    fn clear(&mut self, id: &ProfileId) -> Fallible<()> {
        self.profiles.remove(id);
        self.save()?;
        Ok(())
    }

    fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        // TODO how to implement this?
        unimplemented!()
    }
}
