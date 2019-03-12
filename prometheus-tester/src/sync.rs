use failure::Fallible;
use log::*;

use osg::model::{ProfileData, ProfileId};
use osg::repo::ProfileRepository;

use crate::{state::State, vault::Vault};

pub fn synchronize(state: &mut State, repo: &mut ProfileRepository) -> Fallible<()> {
    let vault = Vault::new(&state.vault_seed())?;
    let mut id_map = std::collections::HashMap::<usize, ProfileId>::with_capacity(state.len());

    for (idx, user) in state.into_iter().enumerate() {
        let id = vault.profile_id(idx)?;
        id_map.insert(idx, id.clone());

        let mut profile = match repo.get(&id) {
            Ok(profile) => profile,
            Err(_e) => ProfileData::empty(&id),
        };

        for peer in user.into_iter() {
            let peer_id = &id_map[peer];
            if profile
                .links
                .iter()
                .find(|l| l.peer_profile == *peer_id)
                .is_none()
            {
                profile.create_link(peer_id);
                info!("Re-created link {}->{}: {}->{}", idx, peer, id, peer_id);
            }
        }

        repo.set(id, profile)?;
    }
    Ok(())
}
