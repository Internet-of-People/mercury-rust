use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use failure::{bail, format_err, Fallible};
use futures::prelude::*;
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
use keyvault::PublicKey as KeyVaultPublicKey;

// TODO consider authorization: should we require signatures here or leave it to a different layer?
/// A whole network of storage nodes, potentially with internal routing and sharding
pub trait DistributedPublicProfileRepository {
    fn get_public(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData>;
    fn set_public(&mut self, profile: PublicProfileData) -> AsyncFallible<()>;
    fn clear_public_local(&mut self, key: &PublicKey) -> AsyncFallible<()>;

    // TODO implement efficient loading based on hints
    // /// Same as load(), but also contains hints for resolution, therefore it's more efficient than load(id)
    // ///
    // /// The `url` may contain
    // /// * ProfileID (mandatory)
    // /// * some profile metadata (for user experience enhancement) (big fat warning should be thrown if it does not match the latest info)
    // /// * ProfileID of its home server
    // /// * last known multiaddress(es) of its home server
    // fn resolve(&self, url: &str) -> AsyncResult<Profile, Error>;

    // TODO notifications on profile updates should be possible
}

// TODO consider authorization: should we require signatures here or leave it to a different layer?
pub trait PrivateProfileRepository {
    fn get(&self, id: &ProfileId) -> AsyncFallible<PrivateProfileData>;
    fn set(&mut self, profile: PrivateProfileData) -> AsyncFallible<()>;
    fn clear(&mut self, key: &PublicKey) -> AsyncFallible<()>;
}

pub trait LocalProfileRepository: PrivateProfileRepository {
    // NOTE similar to set() but without version check, must be able to revert to a previous version
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()>;
}

// TODO should we merge this with PublicProfileRepository?
pub trait ProfileExplorer {
    fn fetch(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData>;
    fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;
    // fn list(&self, /* TODO what filter criteria should we have here? */ ) -> HomeStream<Profile,String>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InMemoryProfileRepository {
    profiles: HashMap<ProfileId, PrivateProfileData>,
}

impl InMemoryProfileRepository {
    pub fn new() -> Self {
        Self { profiles: Default::default() }
    }

    fn put(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        if let Some(old_profile) = self.profiles.get(&profile.id()) {
            if old_profile.version() > profile.version()
                || (old_profile.version() == profile.version() && *old_profile != profile)
            {
                bail!("Version must increase on profile change");
            }
        }
        self.profiles.insert(profile.id(), profile);
        Ok(())
    }

    fn remove(&mut self, key: &PublicKey) -> Fallible<()> {
        let id = key.key_id();
        let profile =
            self.profiles.get(&id).ok_or_else(|| format_err!("Profile not found: {}", key))?;
        self.put(PrivateProfileData::tombstone(key, profile.version()))
    }
}

impl Default for InMemoryProfileRepository {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE normally public and private repositories should not be mixed.
//      We do it here because InMemoryProfileRepository is created for testing, not real usage.
impl DistributedPublicProfileRepository for InMemoryProfileRepository {
    fn get_public(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData> {
        let res =
            (self as &PrivateProfileRepository).get(id).map(|prof_ref| prof_ref.public_data());
        Box::new(res)
    }

    fn set_public(&mut self, profile: PublicProfileData) -> AsyncFallible<()> {
        let private_profile = PrivateProfileData::new(profile, vec![]);
        let res = (self as &mut PrivateProfileRepository).set(private_profile);
        Box::new(res)
    }

    fn clear_public_local(&mut self, key: &PublicKey) -> AsyncFallible<()> {
        let res = (self as &mut PrivateProfileRepository).clear(key);
        Box::new(res)
    }
}

impl PrivateProfileRepository for InMemoryProfileRepository {
    fn get(&self, id: &ProfileId) -> AsyncFallible<PrivateProfileData> {
        let res = self
            .profiles
            .get(id)
            .map(|prof_ref| prof_ref.to_owned())
            .ok_or_else(|| format_err!("Profile not found: {}", id));
        Box::new(res.into_future())
    }

    fn set(&mut self, profile: PrivateProfileData) -> AsyncFallible<()> {
        let res = self.put(profile);
        Box::new(res.into_future())
    }

    fn clear(&mut self, key: &PublicKey) -> AsyncFallible<()> {
        let res = self.remove(key);
        Box::new(res.into_future())
    }
}

impl LocalProfileRepository for InMemoryProfileRepository {
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        self.put(profile)
    }
}

impl ProfileExplorer for InMemoryProfileRepository {
    fn fetch(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData> {
        (self as &DistributedPublicProfileRepository).get_public(id)
    }
    fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!() // TODO
    }
}

#[derive(Debug)]
pub struct FileProfileRepository {
    filename: PathBuf,
}

impl FileProfileRepository {
    // TODO clean up the name chaos here for new(), from, load, store, save
    pub fn new(filename: &PathBuf) -> Fallible<Self> {
        if let Err(_e) = Self::from(filename) {
            info!("Failed to load profile repository from {:?}, initializing one there", filename);
            Self::store(filename, InMemoryProfileRepository::default())?;
        }

        Ok(Self { filename: filename.to_owned() })
    }

    fn from(filename: &PathBuf) -> Fallible<InMemoryProfileRepository> {
        trace!("Loading profile repository from {:?}", filename);
        let repo_file = File::open(filename)?;
        let repo: InMemoryProfileRepository = bincode::deserialize_from(repo_file)?;
        Ok(repo)
    }

    fn store(filename: &PathBuf, mem_repo: InMemoryProfileRepository) -> Fallible<()> {
        trace!("Saving profile repository to {:?}", filename);
        if let Some(repo_dir) = filename.parent() {
            // TODO should we check here first if it already exists?
            trace!("Recursively Creating directory {:?}", repo_dir);
            std::fs::create_dir_all(repo_dir)?;
        }

        let repo_file = File::create(filename)?;
        bincode::serialize_into(repo_file, &mem_repo)?;
        Ok(())
    }

    fn load(&self) -> Fallible<InMemoryProfileRepository> {
        Self::from(&self.filename)
    }

    fn save(&self, mem_repo: InMemoryProfileRepository) -> Fallible<()> {
        Self::store(&self.filename, mem_repo)
    }
}

impl DistributedPublicProfileRepository for FileProfileRepository {
    fn get_public(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData> {
        let res = self.load().and_then(|mem_repo| mem_repo.get_public(id).wait());
        Box::new(res.into_future())
    }

    fn set_public(&mut self, profile: PublicProfileData) -> AsyncFallible<()> {
        let res = self.load().and_then(|mut mem_repo| {
            mem_repo.set_public(profile).wait()?;
            self.save(mem_repo)
        });
        Box::new(res.into_future())
    }

    fn clear_public_local(&mut self, key: &PublicKey) -> AsyncFallible<()> {
        let res = self.load().and_then(|mut mem_repo| {
            mem_repo.clear_public_local(key).wait()?;
            self.save(mem_repo)
        });
        Box::new(res.into_future())
    }
}

impl PrivateProfileRepository for FileProfileRepository {
    fn get(&self, id: &ProfileId) -> AsyncFallible<PrivateProfileData> {
        let res = self.load().and_then(|mem_repo| mem_repo.get(id).wait());
        Box::new(res.into_future())
    }

    fn set(&mut self, profile: PrivateProfileData) -> AsyncFallible<()> {
        let res = self.load().and_then(|mut mem_repo| {
            mem_repo.set(profile).wait()?;
            self.save(mem_repo)
        });
        Box::new(res.into_future())
    }

    fn clear(&mut self, key: &PublicKey) -> AsyncFallible<()> {
        let res = self.load().and_then(|mut mem_repo| {
            mem_repo.clear(key).wait()?;
            self.save(mem_repo)
        });
        Box::new(res.into_future())
    }
}

impl LocalProfileRepository for FileProfileRepository {
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.put(profile)?;
        self.save(mem_repo)
    }
}

impl ProfileExplorer for FileProfileRepository {
    fn fetch(&self, id: &ProfileId) -> AsyncFallible<PublicProfileData> {
        let res = self.load().and_then(move |mem_repo| mem_repo.fetch(id).wait());
        Box::new(res.into_future())
    }
    fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!() // TODO
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use keyvault::PublicKey as KeyVaultPublicKey;

    #[test]
    fn test_local_repository() -> Fallible<()> {
        let tmp_file = std::env::temp_dir().join("local_repo_test.dat");
        let mut repo = FileProfileRepository::new(&tmp_file)?;

        let my_pubkey = PublicKey::from_str("PezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb")?;
        //let my_id = ProfileId::from_str("IezbeWGSY2dqcUBqT8K7R14xr")?;
        let my_id = my_pubkey.key_id();
        let mut my_data = PrivateProfileData::empty(&my_pubkey);
        repo.set(my_data.clone()).wait()?;

        let peer_pubkey = PublicKey::from_str("PezFVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z")?;
        //let peer_id = ProfileId::from_str("Iez25N5WZ1Q6TQpgpyYgiu9gTX")?;
        let peer_id = peer_pubkey.key_id();
        let peer_data = PrivateProfileData::empty(&peer_pubkey);
        repo.set(peer_data.clone()).wait()?;

        let mut me = repo.get(&my_id).wait()?;
        let peer = repo.get(&peer_id).wait()?;
        assert_eq!(me, my_data);
        assert_eq!(peer, peer_data);

        let attr_id = "1 2 3".to_owned();
        let attr_val = "one two three".to_owned();
        my_data.mut_public_data().set_attribute(attr_id, attr_val);
        let _link = my_data.mut_public_data().create_link(&peer_id);
        my_data.mut_public_data().increase_version();
        repo.set(my_data.clone()).wait()?;
        me = repo.get(&my_id).wait()?;
        assert_eq!(me, my_data);
        assert_eq!(me.version(), 2);
        assert_eq!(me.public_data().attributes().len(), 1);
        assert_eq!(me.public_data().links().len(), 1);

        repo.clear(&my_pubkey).wait()?;
        me = repo.get(&my_id).wait()?;
        assert_eq!(
            me,
            PrivateProfileData::new(
                PublicProfileData::new(my_pubkey, 3, Default::default(), Default::default()),
                vec![]
            )
        );

        std::fs::remove_file(&tmp_file)?;

        Ok(())
    }
}
