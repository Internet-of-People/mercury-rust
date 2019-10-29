use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use async_trait::async_trait;
use failure::{bail, format_err, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
use keyvault::PublicKey as KeyVaultPublicKey;

// TODO consider authorization: should we require signatures here or leave it to a different layer?
/// A whole network of storage nodes, potentially with internal routing and sharding
#[async_trait(?Send)]
pub trait DistributedPublicProfileRepository {
    async fn get_public(&self, id: &ProfileId) -> Fallible<PublicProfileData>;
    async fn set_public(&mut self, profile: PublicProfileData) -> Fallible<()>;
    async fn clear_public_local(&mut self, key: &PublicKey) -> Fallible<()>;

    // TODO implement efficient loading based on hints
    // /// Same as load(), but also contains hints for resolution, therefore it's more efficient than load(id)
    // ///
    // /// The `url` may contain
    // /// * ProfileID (mandatory)
    // /// * some profile metadata (for user experience enhancement) (big fat warning should be thrown if it does not match the latest info)
    // /// * ProfileID of its home server
    // /// * last known multiaddress(es) of its home server
    // async fn resolve(&self, url: &str) -> Fallible<Profile>;

    // TODO notifications on profile updates should be possible
}

// TODO consider authorization: should we require signatures here or leave it to a different layer?
#[async_trait(?Send)]
pub trait PrivateProfileRepository {
    async fn get(&self, id: &ProfileId) -> Fallible<PrivateProfileData>;
    async fn set(&mut self, profile: PrivateProfileData) -> Fallible<()>;
    async fn clear(&mut self, key: &PublicKey) -> Fallible<()>;
}

pub trait LocalProfileRepository: PrivateProfileRepository {
    // NOTE similar to set() but without version check, must be able to revert to a previous version
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()>;
}

#[async_trait(?Send)]
pub trait ProfileExplorer {
    async fn fetch(&self, id: &ProfileId) -> Fallible<PublicProfileData>;
    #[deprecated]
    async fn followers(&self, id: &ProfileId) -> Fallible<Vec<Link>>;
    // async fn list(&self, /* TODO what filter criteria should we have here? */ ) -> Fallible<Vec<Profile>>;
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct InMemoryProfileRepository {
    profiles: HashMap<String, PrivateProfileData>,
}

impl InMemoryProfileRepository {
    pub fn new() -> Self {
        Self { profiles: Default::default() }
    }

    fn put(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        if let Some(old_profile) = self.profiles.get(&profile.id().to_string()) {
            if old_profile.version() > profile.version()
                || (old_profile.version() == profile.version()
                    && old_profile.public_data() != profile.public_data())
            {
                bail!("Version must increase on profile change");
            }
        }
        self.profiles.insert(profile.id().to_string(), profile);
        Ok(())
    }

    fn remove(&mut self, key: &PublicKey) -> Fallible<()> {
        let id = key.key_id();
        let profile_version = self
            .profiles
            .get(&id.to_string())
            .ok_or_else(|| format_err!("Profile not found: {}", key))?
            .version();
        self.put(PrivateProfileData::tombstone(key, profile_version))
    }
}

impl Default for InMemoryProfileRepository {
    fn default() -> Self {
        Self::new()
    }
}

// NOTE normally public and private repositories should not be mixed.
//      We do it here because InMemoryProfileRepository is created for testing, not real usage.
#[async_trait(?Send)]
impl DistributedPublicProfileRepository for InMemoryProfileRepository {
    async fn get_public(&self, id: &ProfileId) -> Fallible<PublicProfileData> {
        let prof_ref = PrivateProfileRepository::get(self, id).await?;
        Ok(prof_ref.public_data())
    }

    async fn set_public(&mut self, profile: PublicProfileData) -> Fallible<()> {
        let private_profile = PrivateProfileData::from_public(profile);
        PrivateProfileRepository::set(self, private_profile).await
    }

    async fn clear_public_local(&mut self, key: &PublicKey) -> Fallible<()> {
        PrivateProfileRepository::clear(self, key).await
    }
}

#[async_trait(?Send)]
impl PrivateProfileRepository for InMemoryProfileRepository {
    async fn get(&self, id: &ProfileId) -> Fallible<PrivateProfileData> {
        self.profiles
            .get(&id.to_string())
            .map(|prof_ref| prof_ref.to_owned())
            .ok_or_else(|| format_err!("Profile not found: {}", id))
    }

    async fn set(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        self.put(profile)
    }

    async fn clear(&mut self, key: &PublicKey) -> Fallible<()> {
        self.remove(key)
    }
}

impl LocalProfileRepository for InMemoryProfileRepository {
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        self.put(profile)
    }
}

#[async_trait(?Send)]
impl ProfileExplorer for InMemoryProfileRepository {
    async fn fetch(&self, id: &ProfileId) -> Fallible<PublicProfileData> {
        DistributedPublicProfileRepository::get_public(self, id).await
    }
    async fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
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
            debug!("Failed to load profile repository from {:?}, initializing one there", filename);
            Self::store(filename, InMemoryProfileRepository::default())?;
        }

        Ok(Self { filename: filename.to_owned() })
    }

    fn from(filename: &PathBuf) -> Fallible<InMemoryProfileRepository> {
        trace!("Loading profile repository from {:?}", filename);
        let repo_file = File::open(filename)?;
        //let repo: InMemoryProfileRepository = bincode::deserialize_from(repo_file)?;
        let repo: InMemoryProfileRepository = serde_json::from_reader(repo_file)?;
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
        //bincode::serialize_into(repo_file, &mem_repo)?;
        serde_json::to_writer(repo_file, &mem_repo)?;
        Ok(())
    }

    fn load(&self) -> Fallible<InMemoryProfileRepository> {
        Self::from(&self.filename)
    }

    fn save(&self, mem_repo: InMemoryProfileRepository) -> Fallible<()> {
        Self::store(&self.filename, mem_repo)
    }
}

#[async_trait(?Send)]
impl DistributedPublicProfileRepository for FileProfileRepository {
    async fn get_public(&self, id: &ProfileId) -> Fallible<PublicProfileData> {
        let mem_repo = self.load()?;
        mem_repo.get_public(id).await
    }

    async fn set_public(&mut self, profile: PublicProfileData) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.set_public(profile).await?;
        self.save(mem_repo)
    }

    async fn clear_public_local(&mut self, key: &PublicKey) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.clear_public_local(key).await?;
        self.save(mem_repo)
    }
}

#[async_trait(?Send)]
impl PrivateProfileRepository for FileProfileRepository {
    async fn get(&self, id: &ProfileId) -> Fallible<PrivateProfileData> {
        let mem_repo = self.load()?;
        mem_repo.get(id).await
    }

    async fn set(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.set(profile).await?;
        self.save(mem_repo)
    }

    async fn clear(&mut self, key: &PublicKey) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.clear(key).await?;
        self.save(mem_repo)
    }
}

impl LocalProfileRepository for FileProfileRepository {
    fn restore(&mut self, profile: PrivateProfileData) -> Fallible<()> {
        let mut mem_repo = self.load()?;
        mem_repo.put(profile)?;
        self.save(mem_repo)
    }
}

#[async_trait(?Send)]
impl ProfileExplorer for FileProfileRepository {
    async fn fetch(&self, id: &ProfileId) -> Fallible<PublicProfileData> {
        let mem_repo = self.load()?;
        mem_repo.fetch(id).await
    }
    async fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!() // TODO
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;
    use keyvault::PublicKey as KeyVaultPublicKey;

    #[tokio::test]
    async fn test_local_repository() -> Fallible<()> {
        let tmp_file = std::env::temp_dir().join("local_repo_test.dat");
        let mut repo = FileProfileRepository::new(&tmp_file)?;

        let my_pubkey = PublicKey::from_str("PezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb")?;
        //let my_id = ProfileId::from_str("IezbeWGSY2dqcUBqT8K7R14xr")?;
        let my_id = my_pubkey.key_id();
        let mut my_data = PrivateProfileData::empty(&my_pubkey);
        repo.set(my_data.clone()).await?;

        let peer_pubkey = PublicKey::from_str("PezFVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z")?;
        //let peer_id = ProfileId::from_str("Iez25N5WZ1Q6TQpgpyYgiu9gTX")?;
        let peer_id = peer_pubkey.key_id();
        let peer_data = PrivateProfileData::empty(&peer_pubkey);
        repo.set(peer_data.clone()).await?;

        let mut me = repo.get(&my_id).await?;
        let peer = repo.get(&peer_id).await?;
        assert_eq!(me, my_data);
        assert_eq!(peer, peer_data);

        let attr_id = "1 2 3".to_owned();
        let attr_val = "one two three".to_owned();
        my_data.mut_public_data().set_attribute(attr_id, attr_val);
        let _link = my_data.mut_public_data().create_link(&peer_id);
        my_data.mut_public_data().increase_version();
        repo.set(my_data.clone()).await?;
        me = repo.get(&my_id).await?;
        assert_eq!(me, my_data);
        assert_eq!(me.version(), 2);
        assert_eq!(me.public_data().attributes().len(), 1);
        assert_eq!(me.public_data().links().len(), 1);

        repo.clear(&my_pubkey).await?;
        me = repo.get(&my_id).await?;
        assert_eq!(
            me,
            PrivateProfileData::from_public(PublicProfileData::new(
                my_pubkey,
                3,
                Default::default(),
                Default::default()
            ),)
        );

        std::fs::remove_file(&tmp_file)?;

        Ok(())
    }
}
