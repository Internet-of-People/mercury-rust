use failure::{err_msg, Fallible};
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
    let mercury = mercury_xsk(&state.vault_seed())?;
    let mut id_map = std::collections::HashMap::<usize, ProfileId>::with_capacity(state.len());
    for (idx, _user) in state.into_iter().enumerate() {
        let id = profile_id(&mercury, idx as i32)?;
        id_map.insert(idx, id);
    }

    for (idx, user) in state.into_iter().enumerate() {
        let id = &id_map[&idx];
        let profile = repo
            .get(id)
            .ok_or_else(|| err_msg("Could not connect to server"))?;

        let links_res = profile.clone().borrow().links();
        match links_res {
            Ok(links) => {
                debug!("Found {}: {}", idx, id);
                for peer in user.into_iter() {
                    let peer_id = &id_map[peer];
                    if links.iter().find(|l| l.peer_profile == *peer_id).is_none() {
                        profile.clone().borrow_mut().create_link(peer_id)?;
                        info!("Re-created link {}->{}: {}->{}", idx, peer, id, peer_id);
                    }
                }
            }
            Err(e) => {
                debug!("Not found {}: {}", idx, e);
                let profile = repo.create(id)?;
                info!("Re-created {}: {}", idx, id);
                for peer in user.into_iter() {
                    let peer_id = &id_map[peer];
                    profile.clone().borrow_mut().create_link(peer_id)?;
                    info!("Re-created link {}->{}: {}->{}", idx, peer, id, peer_id);
                }
            }
        }
    }
    Ok(())
}
