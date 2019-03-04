use failure::{err_msg, Fallible};
use log::*;
use rand::{
    distributions::{Distribution, Uniform, WeightedError},
    seq::SliceRandom,
    RngCore,
};
use std::collections::{BTreeMap, BinaryHeap};

use morpheus_storage::{ProfilePtr, ProfileRepository};

use crate::{state::State, vault::Vault};
use rand::thread_rng;

pub struct Simulation<'a> {
    state: &'a mut State,
    repo: &'a mut ProfileRepository,
    inlinks: BTreeMap<usize, usize>,
    vault: Vault,
}

impl<'a> Simulation<'a> {
    pub fn new(state: &'a mut State, repo: &'a mut ProfileRepository) -> Fallible<Self> {
        let vault = Vault::new(state.vault_seed())?;
        let mut inlinks = BTreeMap::new();
        for user in state.into_iter() {
            for peer in user.into_iter() {
                *inlinks.entry(*peer).or_insert(1usize) += 1;
            }
        }
        Ok(Self {
            state,
            repo,
            inlinks,
            vault,
        })
    }

    pub fn stats(&self) -> Fallible<(usize, usize, Vec<usize>)> {
        let users = self.state.len();
        let (links, bheap) = self.inlinks.iter().fold(
            (0usize, BinaryHeap::<usize>::with_capacity(users)),
            |(mut links, mut bheap), (_idx, followers)| {
                links += *followers;
                bheap.push(*followers);
                (links, bheap)
            },
        );

        let influencers: Vec<usize> = bheap
            .into_sorted_vec()
            .iter()
            .rev()
            .cloned()
            .take(10)
            .collect();

        Ok((users, links, influencers))
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
        let src_user = &mut self.state[idx];

        let mut missing_links = src_user.not_links(profile_count);
        if let Some(pos) = missing_links.iter().position(|x| *x == idx) {
            missing_links.remove(pos); // Removes self-link from the possibilities
        }

        debug!("Node {} can still link to {:?}", idx, missing_links);

        let peer_res = {
            let inlinks = &self.inlinks;
            missing_links.as_slice().choose_weighted(rng, |prospect| {
                1 + *inlinks.get(prospect).unwrap_or(&0usize)
            })
        };
        match peer_res {
            Err(WeightedError::NoItem) => info!("Chosen node {} had all possible links", idx),
            Err(_) => info!("Weight calculation is buggy"),
            Ok(peer) => {
                let id = self.vault.profile_id(idx)?;
                let profile = self
                    .repo
                    .get(&id)
                    .ok_or_else(|| err_msg("Could not connect to server"))?;
                self.create_link(profile, idx, *peer)?;
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
            self.create_link(profile, idx, peer)?;
        }
        Ok(())
    }

    fn create_link(&mut self, profile: ProfilePtr, idx: usize, peer: usize) -> Fallible<()> {
        let peer_id = self.vault.profile_id(peer)?;
        let id = profile.borrow().id();
        profile.borrow_mut().create_link(&peer_id)?;
        self.state[idx].add_link(peer);
        *self.inlinks.entry(peer).or_insert(1usize) += 1;
        info!("Generated link {}->{}: {}->{}", idx, peer, id, peer_id);
        Ok(())
    }
}
