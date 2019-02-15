use failure::{bail, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};

use morpheus_keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey, Seed,
    BIP43_PURPOSE_MERCURY,
};
use morpheus_storage::*;

pub trait ProfileVault {
    fn list(&self) -> Fallible<Vec<ProfileId>>;
    fn create_id(&mut self) -> Fallible<ProfileId>;

    fn get_active(&self) -> Fallible<Option<ProfileId>>;
    fn set_active(&mut self, id: &ProfileId) -> Fallible<()>;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DummyProfileVault {
    pub seed: Seed,
    pub indexes: Vec<i32>,
    pub active_idx: Option<i32>,
}

#[allow(clippy::new_without_default)]
impl DummyProfileVault {
    pub fn new() -> Self {
        info!("Initializing profile vault");

        let seed = Seed::from_bip39("include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already").unwrap();
        let indexes = vec![0, 2, 3];
        Self {
            seed,
            indexes,
            active_idx: Option::None,
        }
    }

    fn mercury_xsk(&self) -> Fallible<EdExtPrivateKey> {
        let master = Ed25519::master(&self.seed);
        master.derive_hardened_child(BIP43_PURPOSE_MERCURY)
    }

    fn profile_id(xsk: &EdExtPrivateKey, idx: i32) -> Fallible<ProfileId> {
        let profile_xsk = xsk.derive_hardened_child(idx)?;
        Ok(profile_xsk.neuter().as_public_key().key_id())
    }
}

impl ProfileVault for DummyProfileVault {
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
}
