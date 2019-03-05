use failure::{err_msg, Fallible};
use log::*;

use morpheus_storage::{ProfileId, ProfileRepository};

use crate::{state::State, vault::Vault};

pub fn synchronize(state: &mut State, repo: &mut ProfileRepository) -> Fallible<()> {
    let vault = Vault::new(&state.vault_seed())?;
    let mut id_map = std::collections::HashMap::<usize, ProfileId>::with_capacity(state.len());

    for (idx, _user) in state.into_iter().enumerate() {
        let id = vault.profile_id(idx)?;
        id_map.insert(idx, id.clone());
        let profile = repo
            .get(&id)
            .ok_or_else(|| err_msg("Could not connect to server"))?;

        let links_res = profile.clone().borrow().links();
        match links_res {
            Ok(_links) => {
                debug!("Found {}: {}", idx, id);
            }
            Err(e) => {
                debug!("Not found {}: {}", idx, e);
                repo.create(&id)?;
                info!("Re-created {}: {}", idx, id);
            }
        }
    }

    for (idx, user) in state.into_iter().enumerate() {
        let id = &id_map[&idx];
        let profile = repo
            .get(id)
            .ok_or_else(|| err_msg("Could not connect to server"))?;

        let links_res = profile.clone().borrow().links();
        match links_res {
            Ok(links) => {
                for peer in user.into_iter() {
                    let peer_id = &id_map[peer];
                    if links.iter().find(|l| l.peer_profile == *peer_id).is_none() {
                        profile.clone().borrow_mut().create_link(peer_id)?;
                        info!("Re-created link {}->{}: {}->{}", idx, peer, id, peer_id);
                    }
                }
            }
            Err(e) => {
                error!("Not found links from just created profile {}: {}", idx, e);
            }
        }
    }
    Ok(())
}
