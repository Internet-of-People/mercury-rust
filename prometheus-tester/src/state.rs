use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::{Index, IndexMut};

use morpheus_keyvault::Seed;

#[derive(Deserialize, Serialize)]
pub struct State {
    seed: Seed,
    users: Vec<User>,
}

impl State {
    pub fn new() -> Self {
        let seed = morpheus_keyvault::Seed::from_bip39("include pear escape sail spy orange cute despair witness trouble sleep torch wire burst unable brass expose fiction drift clock duck oxygen aerobic already").unwrap();
        let users = Default::default();
        Self { seed, users }
    }

    pub fn add_user(&mut self) -> usize {
        let idx = self.users.len();
        let user = User {
            outlinks: Default::default(),
        };
        self.users.push(user);
        idx
    }
}

impl Index<usize> for State {
    type Output = User;
    fn index(&self, index: usize) -> &Self::Output {
        let user_opt = self.users.get(index);
        user_opt.unwrap()
    }
}

impl IndexMut<usize> for State {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let user_opt = self.users.get_mut(index);
        user_opt.unwrap()
    }
}

#[derive(Deserialize, Serialize)]
pub struct User {
    outlinks: BTreeSet<usize>,
}

impl User {
    pub fn add_link(&mut self, peer: usize) -> bool {
        let added = !self.outlinks.contains(&peer);
        self.outlinks.insert(peer);
        added
    }
}
