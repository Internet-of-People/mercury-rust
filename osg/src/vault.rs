use std::fs::File;
use std::path::PathBuf;

use failure::{bail, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
use keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey, Seed,
    BIP43_PURPOSE_MERCURY,
};

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>;
    fn create_id(&mut self) -> Fallible<ProfileId>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&mut self, id: &ProfileId) -> Fallible<()>;

    // TODO this should not be on this interface, adding it here as fast hack for MVP demo
    fn save(&self, filename: &PathBuf) -> Fallible<()>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HdProfileVault {
    pub seed: Seed,
    pub indexes: Vec<i32>,
    pub active_idx: Option<i32>,
    // pub profiles: Vec<ProfileData>, // TODO remove this and use local store instead
}

impl HdProfileVault {
    pub fn create(seed: Seed) -> Self {
        info!("Initializing new vault");
        Self {
            seed,
            indexes: Default::default(),
            active_idx: Option::None,
            // profiles: Default::default(),
        }
    }

    fn mercury_xsk(&self) -> Fallible<EdExtPrivateKey> {
        let master = Ed25519::master(&self.seed);
        master.derive_hardened_child(BIP43_PURPOSE_MERCURY)
    }

    fn profile_id(xsk: &EdExtPrivateKey, idx: i32) -> Fallible<ProfileId> {
        let profile_xsk = xsk.derive_hardened_child(idx)?;
        let key_id = profile_xsk.neuter().as_public_key().key_id();
        Ok(key_id.into())
    }

    pub fn load(filename: &PathBuf) -> Fallible<Self> {
        trace!("Loading profile vault from {:?}", filename);
        let vault_file = File::open(filename)?;
        let vault: Self = serde_json::from_reader(&vault_file)?;
        Ok(vault)
    }
}

impl ProfileVault for HdProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>> {
        let mercury_xsk = self.mercury_xsk()?;
        self.indexes
            .iter()
            .try_fold(Vec::with_capacity(self.indexes.len()), |mut v, idx| {
                let profile_id = Self::profile_id(&mercury_xsk, *idx)?;
                v.push(profile_id);
                Ok(v)
            })
    }

    fn create_id(&mut self) -> Fallible<ProfileId> {
        let next_idx = self.indexes.iter().cloned().max().unwrap_or(-1) + 1;
        let xsk = self.mercury_xsk()?;
        let profile_id = Self::profile_id(&xsk, next_idx)?;
        self.indexes.push(next_idx);
        // self.profiles.push(ProfileData::empty(&profile_id));
        debug!("Setting active profile to {}", profile_id);
        self.active_idx = Option::Some(next_idx);
        Ok(profile_id)
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        if let Some(idx) = self.active_idx {
            let xsk = self.mercury_xsk()?;
            Ok(Option::Some(Self::profile_id(&xsk, idx)?))
        } else {
            Ok(Option::None)
        }
    }

    fn set_active(&mut self, id: &ProfileId) -> Fallible<()> {
        if let Some(pos) = self
            .list()?
            .iter()
            .position(|candidate_id| candidate_id == id)
        {
            self.active_idx = Option::Some(self.indexes[pos]);
            Ok(())
        } else {
            bail!("Profile Id '{}' not found", id)
        }
    }

    fn save(&self, filename: &PathBuf) -> Fallible<()> {
        debug!("Saving profile vault to store state");
        if let Some(vault_dir) = filename.parent() {
            debug!("Recursively Creating directory {:?}", vault_dir);
            std::fs::create_dir_all(vault_dir)?;
        }

        let vault_file = File::create(filename)?;
        serde_json::to_writer_pretty(&vault_file, self)?;
        Ok(())
    }
}
