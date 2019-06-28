use std::fs::File;
use std::path::PathBuf;

use failure::{bail, ensure, err_msg, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;
use keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey as KeyVaultPublicKey,
    Seed, BIP43_PURPOSE_MERCURY,
};

// TODO this should work with MPrivateKey to support any key type,
//      and thus key derivation should be exported to the keyvault::PrivateKey trait
pub struct MercuryProfiles {
    mercury_xsk: EdExtPrivateKey,
}

impl MercuryProfiles {
    pub fn public_key(&self, idx: i32) -> Fallible<PublicKey> {
        let profile_xsk = self.mercury_xsk.derive_hardened_child(idx)?;
        let key = profile_xsk.neuter().as_public_key();
        Ok(key.into())
    }

    pub fn id(&self, idx: i32) -> Fallible<ProfileId> {
        self.public_key(idx).map(|key| key.key_id())
    }
}

pub struct MercurySecrets {
    mercury_xsk: EdExtPrivateKey,
}

impl MercurySecrets {
    pub fn private_key(&self, idx: i32) -> Fallible<PrivateKey> {
        let profile_xsk = self.mercury_xsk.derive_hardened_child(idx)?;
        let key = profile_xsk.as_private_key();
        Ok(key.into())
    }
}

pub type ProfileAlias = String;

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<(ProfileAlias, ProfileId)>>;
    fn create_key(&mut self, alias: ProfileAlias) -> Fallible<PublicKey>;
    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()>;

    fn alias_by_id(&self, id: &ProfileId) -> Fallible<ProfileAlias>;
    fn id_by_alias(&self, alias: &ProfileAlias) -> Fallible<ProfileId>;
    fn set_alias(&mut self, id: ProfileId, alias: ProfileAlias) -> Fallible<()>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&mut self, id: &ProfileId) -> Fallible<()>;

    // TODO these probably should not be here on the long run, list() is enough in most cases.
    //      Used only for restoring all profiles of a vault with gap detection.
    fn profiles(&self) -> Fallible<MercuryProfiles>;
    fn len(&self) -> usize;

    // TODO this should not be on this interface on the long run.
    //      Used for saving vault state on a change.
    fn save(&self, filename: &PathBuf) -> Fallible<()>;
}

pub const GAP: usize = 20;

#[derive(Debug, Deserialize, Serialize)]
pub struct HdProfileVault {
    pub seed: Seed,
    pub next_idx: i32,
    pub active_idx: Option<i32>,
    pub aliases: Vec<String>,
}

impl HdProfileVault {
    pub fn create(seed: Seed) -> Self {
        info!("Initializing new vault");
        Self { seed, next_idx: Default::default(), active_idx: Option::None, aliases: vec![] }
    }

    pub fn load(filename: &PathBuf) -> Fallible<Self> {
        trace!("Loading profile vault from {:?}", filename);
        let vault_file = File::open(filename)?;
        let vault: Self = serde_json::from_reader(&vault_file)?;
        //let vault: Self = bincode::deserialize_from(vault_file)?;
        ensure!(vault.next_idx >= 0, "next_idx cannot be negative");
        if let Some(active) = vault.active_idx {
            ensure!(active >= 0, "active_idx cannot be negative");
            ensure!(active < vault.next_idx, "active_idx cannot exceed last profile index");
        }
        ensure!(vault.next_idx as usize == vault.aliases.len(), "an alias must exist for each id");

        use std::{collections::HashSet, iter::FromIterator};
        let unique_aliases: HashSet<String> = HashSet::from_iter(vault.aliases.iter().cloned());
        ensure!(vault.aliases.len() == unique_aliases.len(), "all aliases must be unique");

        Ok(vault)
    }

    // TODO this should not be exposed, it's more like an implementation detail
    pub fn index_of_id(&self, id: &ProfileId) -> Option<usize> {
        let profiles = self.profiles().ok()?;
        for idx in 0..self.next_idx {
            if profiles.id(idx).ok()? == *id {
                return Some(idx as usize);
            }
        }
        None
    }

    //    fn index_of_alias(&self, alias: &ProfileAlias) -> Option<usize> {
    //        self.list().ok().and_then(|v| v.iter().position(|pair| pair.0 == *alias))
    //    }

    fn list_range(&self, range: std::ops::Range<i32>) -> Fallible<Vec<(ProfileAlias, ProfileId)>> {
        let profiles = self.profiles()?;
        let mut v = Vec::with_capacity(range.len());
        for idx in range {
            let profile_id = profiles.id(idx)?;
            let alias = self.alias_by_id(&profile_id)?;
            v.push((alias, profile_id));
        }
        Ok(v)
    }

    fn mercury_xsk(&self) -> Fallible<EdExtPrivateKey> {
        let master = Ed25519::master(&self.seed);
        master.derive_hardened_child(BIP43_PURPOSE_MERCURY)
    }

    // TODO this should be exposed in a safer way, at least protected by some password
    pub fn secrets(&self) -> Fallible<MercurySecrets> {
        Ok(MercurySecrets { mercury_xsk: self.mercury_xsk()? })
    }
}

impl ProfileVault for HdProfileVault {
    fn len(&self) -> usize {
        self.next_idx as usize
    }

    fn profiles(&self) -> Fallible<MercuryProfiles> {
        Ok(MercuryProfiles { mercury_xsk: self.mercury_xsk()? })
    }

    fn list(&self) -> Fallible<Vec<(ProfileAlias, ProfileId)>> {
        self.list_range(0..self.next_idx)
    }

    fn create_key(&mut self, alias: ProfileAlias) -> Fallible<PublicKey> {
        ensure!(!self.aliases.contains(&alias), "the specified alias must be unique");
        ensure!(self.aliases.len() == self.next_idx as usize, "an alias must exist for each id");
        let key = self.profiles()?.public_key(self.next_idx)?;
        self.active_idx = Option::Some(self.next_idx);
        self.aliases.push(alias);
        self.next_idx += 1;
        debug!("Setting active profile to {}", key.key_id());
        Ok(key)
    }

    fn restore_id(&mut self, id: &ProfileId) -> Fallible<()> {
        if self.index_of_id(id).is_some() {
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

    fn alias_by_id(&self, id: &ProfileId) -> Fallible<ProfileAlias> {
        let idx = self.index_of_id(id).ok_or_else(|| err_msg("id is not found in vault"))?;
        self.aliases
            .get(idx)
            .map(|v| v.to_owned())
            .ok_or_else(|| err_msg("Implementation error: alias not found for generated id"))
    }

    fn id_by_alias(&self, alias: &ProfileAlias) -> Fallible<ProfileId> {
        let profiles = self.list()?;
        profiles
            .iter()
            .filter_map(|pair| if pair.0 == *alias { Some(pair.1.clone()) } else { None })
            .nth(0)
            .ok_or_else(|| err_msg("alias is not found in vault"))
    }

    fn set_alias(&mut self, id: ProfileId, alias: ProfileAlias) -> Fallible<()> {
        let idx = self.index_of_id(&id).ok_or_else(|| err_msg("id is not found in vault"))?;
        self.aliases
            .get_mut(idx)
            .map(|a| *a = alias)
            .ok_or_else(|| err_msg("Implementation error: alias not found for generated id"))
    }

    fn get_active(&self) -> Fallible<Option<ProfileId>> {
        if let Some(idx) = self.active_idx {
            Ok(Option::Some(self.profiles()?.id(idx)?))
        } else {
            Ok(Option::None)
        }
    }

    fn set_active(&mut self, id: &ProfileId) -> Fallible<()> {
        if let Some(idx) = self.index_of_id(id) {
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
        serde_json::to_writer_pretty(&vault_file, self)?;
        //bincode::serialize_into(vault_file, self)?;
        Ok(())
    }
}
