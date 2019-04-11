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

// TODO this should work with MPrivateKey to support any key type,
//      and thus key derivation should be exported to the keyvault::PrivateKey trait
pub struct MercuryProfiles {
    mercury_xsk: EdExtPrivateKey,
}

impl MercuryProfiles {
    pub fn id(&self, idx: i32) -> Fallible<ProfileId> {
        let profile_xsk = self.mercury_xsk.derive_hardened_child(idx)?;
        let key_id = profile_xsk.neuter().as_public_key().key_id();
        Ok(key_id.into())
    }
}

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>;
    fn create_id(&mut self) -> Fallible<ProfileId>;
    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&mut self, id: &ProfileId) -> Fallible<()>;

    // TODO these probably should not be on this interface on the long run.
    //      Used only for restoring all profiles of a vault with gap detection.
    fn profiles(&self) -> Fallible<MercuryProfiles>;
    fn len(&self) -> usize;

    // TODO this should not be on this interface on the long run.
    //      Used for saving vault state when CLI finished.
    fn save(&self, filename: &PathBuf) -> Fallible<()>;
}

pub const GAP: usize = 20;

#[derive(Debug, Deserialize, Serialize)]
pub struct HdProfileVault {
    pub seed: Seed,
    pub next_idx: i32,
    pub active_idx: Option<i32>,
}

impl HdProfileVault {
    pub fn create(seed: Seed) -> Self {
        info!("Initializing new vault");
        Self { seed, next_idx: Default::default(), active_idx: Option::None }
    }

    fn index_of(&self, id: &ProfileId) -> Option<usize> {
        self.list().ok().and_then(|v| v.iter().position(|candidate_id| candidate_id == id))
    }

    pub fn load(filename: &PathBuf) -> Fallible<Self> {
        trace!("Loading profile vault from {:?}", filename);
        let vault_file = File::open(filename)?;
        //let vault: Self = serde_json::from_reader(&vault_file)?;
        let vault: Self = bincode::deserialize_from(vault_file)?;
        ensure!(vault.next_idx >= 0, "next_idx cannot be negative");
        if let Some(active) = vault.active_idx {
            ensure!(active >= 0, "active_idx cannot be negative");
            ensure!(active < vault.next_idx, "active_idx cannot exceed last profile index");
        }

        Ok(vault)
    }

    fn list_range(&self, range: std::ops::Range<i32>) -> Fallible<Vec<ProfileId>> {
        let profiles = self.profiles()?;
        let mut v = Vec::with_capacity(range.len());
        for idx in range {
            let profile_id = profiles.id(idx)?;
            v.push(profile_id);
        }
        Ok(v)
    }
}

impl ProfileVault for HdProfileVault {
    fn len(&self) -> usize {
        self.next_idx as usize
    }

    fn profiles(&self) -> Fallible<MercuryProfiles> {
        let master = Ed25519::master(&self.seed);
        let mercury_xsk = master.derive_hardened_child(BIP43_PURPOSE_MERCURY)?;
        Ok(MercuryProfiles { mercury_xsk })
    }

    fn list(&self) -> Fallible<Vec<ProfileId>> {
        self.list_range(0..self.next_idx)
    }

    fn create_id(&mut self) -> Fallible<ProfileId> {
        let profile_id = self.profiles()?.id(self.next_idx)?;
        self.active_idx = Option::Some(self.next_idx);
        self.next_idx += 1;
        debug!("Setting active profile to {}", profile_id);
        Ok(profile_id)
    }

    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()> {
        if self.index_of(id).is_some() {
            trace!("Profile id {} is already present in the vault", id);
            return Ok(());
        }

        trace!(
            "Profile id {} is not contained yet, trying to find it from index {} with {} gap",
            id,
            self.next_idx,
            GAP
        );

        let profiles = self.profiles()?;
        for idx in self.next_idx..self.next_idx + GAP as i32 {
            if *id == profiles.id(idx)? {
                trace!("Profile id {} is found at key index {}", id, idx);
                self.next_idx = idx + 1;
                return Ok(());
            }
        }

        bail!("{} is not owned by this seed", id);
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        if let Some(idx) = self.active_idx {
            Ok(Option::Some(self.profiles()?.id(idx)?))
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
