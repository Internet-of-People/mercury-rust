use failure::Fallible;
use log::*;

use morpheus_keyvault::{
    ed25519::{Ed25519, EdExtPrivateKey},
    ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey, Seed,
    BIP43_PURPOSE_MERCURY,
};
use morpheus_storage::{ProfileId, ProfileRepository};

use crate::state::State;

fn mercury_xsk(seed: &Seed) -> Fallible<EdExtPrivateKey> {
    let master = Ed25519::master(seed);
    master.derive_hardened_child(BIP43_PURPOSE_MERCURY)
}

fn profile_id(xsk: &EdExtPrivateKey, idx: i32) -> Fallible<ProfileId> {
    let profile_xsk = xsk.derive_hardened_child(idx)?;
    let key_id = profile_xsk.neuter().as_public_key().key_id();
    Ok(key_id.into())
}

pub fn synchronize(state: &mut State, repo: &mut ProfileRepository) -> Fallible<()> {
    let mercury = mercury_xsk(&state.seed())?;
    for (i, _user) in state.into_iter().enumerate() {
        let id = profile_id(&mercury, i as i32)?;
        if let Some(_profile_ptr) = repo.get(&id) {
            debug!("Found {}: {}", i, id);
        // sync links for profile_ptr
        } else {
            info!("Re-creating {}: {}", i, id);
        }
    }
    Ok(())
}
