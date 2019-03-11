use std::collections::{BTreeMap, BinaryHeap};
use std::fmt;

use failure::Fallible;
use log::*;
use rand::{
    distributions::{Distribution, Uniform, WeightedError},
    seq::SliceRandom,
};

use crate::{state::State, vault::Vault};
use osg::profile::ProfilePtr;
use osg::repo::ProfileRepository;

#[derive(Clone)]
pub struct InlinkCount {
    idx: usize,
    inlinks: usize,
}

impl PartialEq<InlinkCount> for InlinkCount {
    fn eq(&self, other: &InlinkCount) -> bool {
        self.inlinks.eq(&other.inlinks)
    }
}
impl Eq for InlinkCount {}
impl PartialOrd<InlinkCount> for InlinkCount {
    fn partial_cmp(&self, other: &InlinkCount) -> Option<std::cmp::Ordering> {
        self.inlinks.partial_cmp(&other.inlinks)
    }
}
impl Ord for InlinkCount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inlinks.cmp(&other.inlinks)
    }
}
impl fmt::Display for InlinkCount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}*{}", self.idx, self.inlinks)
    }
}

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
                *inlinks.entry(*peer).or_insert(0usize) += 1;
            }
        }
        Ok(Self {
            state,
            repo,
            inlinks,
            vault,
        })
    }

    pub fn stats(&self) -> Fallible<(usize, usize, usize, Vec<InlinkCount>)> {
        let steps = self.state.steps();
        let users = self.state.len();
        let (links, bheap) = self.inlinks.iter().fold(
            (0usize, BinaryHeap::<InlinkCount>::with_capacity(users)),
            |(mut links, mut bheap), (&idx, &inlinks)| {
                links += inlinks;
                bheap.push(InlinkCount { idx, inlinks });
                (links, bheap)
            },
        );

        let influencers: Vec<InlinkCount> = bheap
            .into_sorted_vec()
            .iter()
            .rev()
            .cloned()
            .take(10)
            .collect();

        Ok((steps, users, links, influencers))
    }

    pub fn step(&mut self) -> Fallible<()> {
        let weight_create_profile = 5; // TODO config
        let weight_update_profile = self.state.len();
        let dist = Uniform::new(0, weight_create_profile + weight_update_profile);
        if dist.sample(self.state.rand()) >= weight_create_profile {
            self.update_profile()?;
        } else {
            self.create_profile()?;
        }
        self.state.add_step();
        Ok(())
    }

    fn update_profile(&mut self) -> Fallible<()> {
        let profile_count = self.state.len();

        let src_dist = Uniform::new(0, profile_count);
        let idx = src_dist.sample(self.state.rand());

        self.add_link_to_user(idx)
    }

    fn create_profile(&mut self) -> Fallible<()> {
        // TODO this should be removed after ProfileRepository trait is settled
        use osg::profile::LocalProfile;
        use std::cell::RefCell;
        use std::rc::Rc;

        let old_profile_count = self.state.len();

        let idx = self.state.add_user();
        let id = self.vault.profile_id(idx)?;

        let profile = Rc::new(RefCell::new(LocalProfile::new(&id)));
        self.repo.set(&id, profile)?;
        info!("Generated profile {}: {}", idx, id);

        if old_profile_count > 0 {
            self.add_link_to_user(idx)
        } else {
            Ok(())
        }
    }

    fn add_link_to_user(&mut self, idx: usize) -> Fallible<()> {
        let profile_count = self.state.len();
        let src_user = &mut self.state[idx];
        let mut missing_links = src_user.not_links(profile_count);
        if let Some(pos) = missing_links.iter().position(|x| *x == idx) {
            missing_links.remove(pos); // Removes self-link from the possibilities
        }

        debug!("Node {} can still link to {:?}", idx, missing_links);

        let rand = self.state.rand();
        let inlinks = &self.inlinks;
        let peer_res = missing_links
            .as_slice()
            .choose_weighted(rand, |prospect| Self::weight(inlinks, *prospect));
        match peer_res {
            Err(WeightedError::NoItem) => info!("Chosen node {} had all possible links", idx),
            Err(_) => info!("Weight calculation is buggy"),
            Ok(peer) => {
                let id = self.vault.profile_id(idx)?;
                let profile = self.repo.get(&id)?;
                self.create_link(profile, idx, *peer)?;
            }
        };
        Ok(())
    }

    fn weight(inlinks: &BTreeMap<usize, usize>, prospect: usize) -> f64 {
        match inlinks.get(&prospect) {
            Some(followers) => (*followers as f64).powi(2),
            None => 0.1f64,
        }
    }

    fn create_link(&mut self, profile: ProfilePtr, idx: usize, peer: usize) -> Fallible<()> {
        let peer_id = self.vault.profile_id(peer)?;
        let id = profile.borrow().id();
        profile.borrow_mut().create_link(&peer_id)?;
        self.state[idx].add_link(peer);
        *self.inlinks.entry(peer).or_insert(0usize) += 1;
        info!("Generated link {}->{}: {}->{}", idx, peer, id, peer_id);
        Ok(())
    }
}
