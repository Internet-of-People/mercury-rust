use failure::Fallible;

use morpheus_keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey, Seed,
    BIP43_PURPOSE_MERCURY,
};
use morpheus_storage::ProfileId;

pub struct Vault {
    mercury_xsk: EdExtPrivateKey,
}

impl Vault {
    pub fn new(seed: &Seed) -> Fallible<Self> {
        let master = Ed25519::master(seed);
        let mercury_xsk = master.derive_hardened_child(BIP43_PURPOSE_MERCURY)?;
        Ok(Self { mercury_xsk })
    }

    pub fn profile_id(&self, idx: usize) -> Fallible<ProfileId> {
        let profile_xsk = self.mercury_xsk.derive_hardened_child(idx as i32)?;
        let key_id = profile_xsk.neuter().as_public_key().key_id();
        Ok(key_id.into())
    }
}
