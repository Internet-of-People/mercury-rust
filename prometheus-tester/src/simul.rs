use failure::Fallible;
use rand::{
    distributions::{Distribution, Uniform},
    RngCore, SeedableRng,
};
use rand_chacha::ChaChaRng;

use morpheus_storage::ProfileRepository;

use crate::{state::State, vault::Vault};

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
        let mut rng = ChaChaRng::seed_from_u64(0); // TODO state + config
        let weight_create_profile = 5; // TODO config
        let weight_update_profile = self.state.len();
        let dist = Uniform::new(0, weight_create_profile + weight_update_profile);
        if dist.sample(&mut rng) >= weight_create_profile {
            self.update_profile(&mut rng)
        } else {
            self.create_profile(&mut rng)
        }
    }

    fn update_profile(&mut self, rng: &mut RngCore) -> Fallible<()> {
        Ok(())
    }

    fn create_profile(&mut self, rng: &mut RngCore) -> Fallible<()> {
        let old_profile_count = self.state.len();

        let idx = self.state.add_user();
        let id = self.vault.profile_id(idx)?;
        let profile = self.repo.create(&id)?;

        if old_profile_count > 0 {
            let dist = Uniform::new(0, old_profile_count);
            let peer = dist.sample(rng);

            let peer_id = self.vault.profile_id(peer)?;
            profile.borrow_mut().create_link(&peer_id)?;
            self.state[idx].add_link(peer);
        }
        Ok(())
    }
}
