use std::fs::File;
use std::path::PathBuf;

use failure::{bail, ensure, Fallible};
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
    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&mut self, id: &ProfileId) -> Fallible<()>;

    // TODO this should not be on this interface, adding it here as fast hack for MVP demo
    fn save(&self, filename: &PathBuf) -> Fallible<()>;
}

const GAP: i32 = 20;

#[derive(Debug, Deserialize, Serialize)]
pub struct HdProfileVault {
    pub seed: Seed,
    pub next_idx: i32,
    pub active_idx: Option<i32>,
}

impl HdProfileVault {
    pub fn create(seed: Seed) -> Self {
        info!("Initializing new vault");
        Self {
            seed,
            next_idx: Default::default(),
            active_idx: Option::None,
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

    fn index_of(&self, id: &ProfileId) -> Option<usize> {
        self.list()
            .ok()
            .and_then(|v| v.iter().position(|candidate_id| candidate_id == id))
    }

    pub fn load(filename: &PathBuf) -> Fallible<Self> {
        trace!("Loading profile vault from {:?}", filename);
        let vault_file = File::open(filename)?;
        //let vault: Self = serde_json::from_reader(&vault_file)?;
        let vault: Self = bincode::deserialize_from(vault_file)?;
        ensure!(vault.next_idx >= 0, "next_idx cannot be negative");
        if let Some(active) = vault.active_idx {
            ensure!(active >= 0, "active_idx cannot be negative");
            ensure!(
                active < vault.next_idx,
                "active_idx cannot exceed last profile index"
            );
        }

        Ok(vault)
    }
}

impl ProfileVault for HdProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>> {
        let mercury_xsk = self.mercury_xsk()?;
        let mut v = Vec::with_capacity(self.next_idx as usize);
        for idx in 0..self.next_idx {
            let profile_id = Self::profile_id(&mercury_xsk, idx)?;
            v.push(profile_id);
        }
        Ok(v)
    }

    fn create_id(&mut self) -> Fallible<ProfileId> {
        let xsk = self.mercury_xsk()?;
        let profile_id = Self::profile_id(&xsk, self.next_idx)?;
        self.active_idx = Option::Some(self.next_idx);
        self.next_idx += 1;
        debug!("Setting active profile to {}", profile_id);
        Ok(profile_id)
    }

    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()> {
        if self.index_of(id).is_some() {
            return Ok(());
        }

        for _i in 0..GAP {
            if *id == self.create_id()? {
                return Ok(());
            }
        }

        bail!("{} is not owned by this seed", id);
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
        if let Some(idx) = self.index_of(id) {
            self.active_idx = Option::Some(idx as i32);
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
        //serde_json::to_writer_pretty(&vault_file, self)?;
        bincode::serialize_into(vault_file, self)?;
        Ok(())
    }
}
