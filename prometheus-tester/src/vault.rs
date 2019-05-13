use failure::Fallible;

use keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey as KeyVaultPublicKey,
    Seed, BIP43_PURPOSE_MERCURY,
};
use osg::model::{ProfileId, PublicKey};

pub struct Vault {
    mercury_xsk: EdExtPrivateKey,
}

impl Vault {
    pub fn new(seed: &Seed) -> Fallible<Self> {
        let master = Ed25519::master(seed);
        let mercury_xsk = master.derive_hardened_child(BIP43_PURPOSE_MERCURY)?;
        Ok(Self { mercury_xsk })
    }

    pub fn public_key(&self, idx: usize) -> Fallible<PublicKey> {
        let profile_xsk = self.mercury_xsk.derive_hardened_child(idx as i32)?;
        let key_id = profile_xsk.neuter().as_public_key();
        Ok(key_id.into())
    }

    pub fn profile_id(&self, idx: usize) -> Fallible<ProfileId> {
        self.public_key(idx).map(|pubkey| pubkey.key_id())
    }
}
