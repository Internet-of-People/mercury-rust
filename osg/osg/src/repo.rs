use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use failure::{bail, err_msg, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData>;
    fn set(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()>;
    // clear up links and attributes to leave an empty tombstone in place of the profile.
    fn clear(&mut self, id: &ProfileId) -> Fallible<()>;
}

pub trait LocalProfileRepository: ProfileRepository {
    // NOTE similar to set() but without version check, must be able to revert to a previous version
    fn restore(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()>;
}

pub trait ProfileExplorer {
    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileProfileRepository {
    profiles: HashMap<ProfileId, ProfileData>,
    #[serde(skip)]
    filename: PathBuf,
}

impl FileProfileRepository {
    pub fn create(filename: &PathBuf) -> Fallible<Self> {
        let this = Self { profiles: Default::default(), filename: filename.to_owned() };
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

    fn store(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()> {
        self.profiles.insert(id, profile);
        self.save()?;
        Ok(())
    }
}

impl ProfileRepository for FileProfileRepository {
    fn get(&self, id: &ProfileId) -> Fallible<ProfileData> {
        // TODO we probably should also have some nicely typed errors here
        self.profiles
            .get(id)
            .map(|prof_ref| prof_ref.to_owned())
            .ok_or_else(|| err_msg("Profile not found"))
    }

    fn set(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()> {
        if let Some(old_profile) = self.profiles.get(&id) {
            if old_profile.version() > profile.version() {
                bail!("Profile version must monotonically increase");
            }
            if old_profile.version() == profile.version() && *old_profile != profile {
                bail!("Version must increase on profile change");
            }
        }

        self.store(id, profile)
    }

    fn clear(&mut self, id: &ProfileId) -> Fallible<()> {
        let profile = self.get(id)?;
        //self.profiles.remove(id);
        self.set(id.to_owned(), ProfileData::tombstone(id, profile.version()))?;
        Ok(())
    }
}

impl LocalProfileRepository for FileProfileRepository {
    fn restore(&mut self, id: ProfileId, profile: ProfileData) -> Fallible<()> {
        self.store(id, profile)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_local_repository() -> Fallible<()> {
        let tmp_file = std::env::temp_dir().join("local_repo_test.dat");
        let mut repo = FileProfileRepository::create(&tmp_file)?;

        let my_id = ProfileId::from_str("IezbeWGSY2dqcUBqT8K7R14xr")?;
        let mut my_data = ProfileData::new(&my_id);
        repo.set(my_id.clone(), my_data.clone())?;

        let peer_id = ProfileId::from_str("Iez25N5WZ1Q6TQpgpyYgiu9gTX")?;
        let peer_data = ProfileData::new(&peer_id);
        repo.set(peer_id.clone(), peer_data.clone())?;

        let mut me = repo.get(&my_id)?;
        let peer = repo.get(&peer_id)?;
        assert_eq!(me, my_data);
        assert_eq!(peer, peer_data);

        let attr_id = "1 2 3".to_owned();
        let attr_val = "one two three".to_owned();
        my_data.set_attribute(attr_id, attr_val);
        let _link = my_data.create_link(&peer_id);
        my_data.increase_version();
        repo.set(my_id.clone(), my_data.clone())?;
        me = repo.get(&my_id)?;
        assert_eq!(me, my_data);
        assert_eq!(me.version(), 2);
        assert_eq!(me.attributes().len(), 1);
        assert_eq!(me.links().len(), 1);

        repo.clear(&my_id)?;
        me = repo.get(&my_id)?;
        assert_eq!(me, ProfileData::create(my_id, 3, Default::default(), Default::default()));

        std::fs::remove_file(&tmp_file)?;

        Ok(())
    }
}
