use failure::Fallible;
use futures::prelude::*;
use log::*;

use did::model::{PrivateProfileData, ProfileId};
use did::repo::PrivateProfileRepository;
use keyvault::PublicKey as KeyVaultPublicKey;

use crate::{state::State, vault::Vault};

pub fn synchronize(state: &mut State, repo: &mut PrivateProfileRepository) -> Fallible<()> {
    let vault = Vault::new(&state.vault_seed())?;
    let mut id_map = std::collections::HashMap::<usize, ProfileId>::with_capacity(state.len());

    info!("Synchronizing profiles");
    for (idx, _user) in state.into_iter().enumerate() {
        let key = vault.public_key(idx)?;
        let id = key.key_id();
        id_map.insert(idx, id.clone());

        if repo.get(&id).wait().is_err() {
            info!("Reconstructing profile {}", id);
            let profile = PrivateProfileData::empty(&key);
            repo.set(profile).wait()?;
        }
    }

    info!("Synchronizing links");
    for (idx, user) in state.into_iter().enumerate() {
        let id = &id_map[&idx];
        let mut profile = repo.get(id).wait()?;
        profile.mut_public_data().increase_version();

        for peer in user.into_iter() {
            let peer_id = &id_map[peer];
            if profile.public_data().links().iter().find(|l| l.peer_profile == *peer_id).is_none() {
                profile.mut_public_data().create_link(peer_id);
                info!("Re-created link {}->{}: {}->{}", idx, peer, id, peer_id);
            }
        }

        repo.set(profile).wait()?;
    }
    Ok(())
}
