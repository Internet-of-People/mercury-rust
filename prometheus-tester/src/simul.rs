use failure::{err_msg, Fallible};
use log::*;
use rand::{
    distributions::{Distribution, Uniform},
    seq::SliceRandom,
    RngCore, SeedableRng,
};
use rand_chacha::ChaChaRng;

use morpheus_storage::ProfileRepository;

use crate::{state::State, vault::Vault};
use rand::thread_rng;

pub struct Simulation<'a> {
    state: &'a mut State,
    repo: &'a mut ProfileRepository,
    vault: Vault,
}

impl<'a> Simulation<'a> {
    pub fn new(state: &'a mut State, repo: &'a mut ProfileRepository) -> Fallible<Self> {
        let vault = Vault::new(state.vault_seed())?;
        Ok(Self { state, repo, vault })
    }

    pub fn step(&mut self) -> Fallible<()> {
        //        let seed = self.state.rand_seed();
        //        let mut rng = ChaChaRng::from_seed(*seed);
        let mut rng = thread_rng();
        let weight_create_profile = 5; // TODO config
        let weight_update_profile = self.state.len();
        let dist = Uniform::new(0, weight_create_profile + weight_update_profile);
        if dist.sample(&mut rng) >= weight_create_profile {
            self.update_profile(&mut rng)?;
        } else {
            self.create_profile(&mut rng)?;
        }
        //*self.state.rand_seed() = (rng as <ChaChaRng as SeedableRng>::Seed).as_mut();
        Ok(())
    }

    fn update_profile(&mut self, rng: &mut RngCore) -> Fallible<()> {
        let profile_count = self.state.len();

        let src_dist = Uniform::new(0, profile_count);
        let idx = src_dist.sample(rng);
        let src_user = &self.state[idx];

        let mut missing_links = src_user.not_links(profile_count);
        if let Some(pos) = missing_links.iter().position(|x| *x == idx) {
            missing_links.remove(pos); // Removes self-link from the possibilities
        }

        debug!("Node {} can still link to {:?}", idx, missing_links);

        match missing_links.as_slice().choose(rng) {
            None => info!("Chosen node {} had all possible links", idx),
            Some(peer) => {
                let id = self.vault.profile_id(idx)?;
                let peer_id = self.vault.profile_id(*peer)?;
                let profile = self
                    .repo
                    .get(&id)
                    .ok_or_else(|| err_msg("Could not connect to server"))?;
                profile.borrow_mut().create_link(&peer_id)?;
                self.state[idx].add_link(*peer);
                info!("Generated link {}->{}: {}->{}", idx, *peer, id, peer_id);
            }
        }

        Ok(())
    }

    fn create_profile(&mut self, rng: &mut RngCore) -> Fallible<()> {
        let old_profile_count = self.state.len();

        let idx = self.state.add_user();
        let id = self.vault.profile_id(idx)?;
        let profile = self.repo.create(&id)?;
        info!("Generated profile {}: {}", idx, id);

        if old_profile_count > 0 {
            let dist = Uniform::new(0, old_profile_count);
            let peer = dist.sample(rng);

            let peer_id = self.vault.profile_id(peer)?;
            profile.borrow_mut().create_link(&peer_id)?;
            self.state[idx].add_link(peer);
            info!("Generated link {}->{}: {}->{}", idx, peer, id, peer_id);
        }
        Ok(())
    }
}
