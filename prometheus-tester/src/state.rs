use failure::Fallible;
use rand::SeedableRng;
use rand_chacha::ChaChaCore;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::{Index, IndexMut};
use rand::ChaChaRng;

pub type RngSeed = <ChaChaCore as SeedableRng>::Seed;

#[derive(Deserialize, Serialize)]
pub struct State {
    vault_seed: morpheus_keyvault::Seed,
    rand_seed: RngSeed,
//    #[serde(with="serde_bytes")]
//    rand_seed: Vec<u8>,
    users: Vec<User>,
}

impl State {
    pub fn new<S: AsRef<str>>(phrase: S) -> Fallible<Self> {
        let vault_seed = morpheus_keyvault::Seed::from_bip39(phrase)?;
        let rand_seed = RngSeed::default(); // TODO config
        let users = Default::default();
        Ok(Self {
            vault_seed,
            rand_seed,
            users,
        })
    }

    pub fn vault_seed(&self) -> &morpheus_keyvault::Seed {
        &self.vault_seed
    }

    pub fn rand_seed(&mut self) -> &mut RngSeed {
        &mut self.rand_seed
    }

    pub fn len(&self) -> usize {
        self.users.len()
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

impl<'a> IntoIterator for &'a State {
    type Item = &'a User;
    type IntoIter = std::slice::Iter<'a, User>;
    fn into_iter(self) -> Self::IntoIter {
        self.users.iter()
    }
}

impl<'a> IntoIterator for &'a mut State {
    type Item = &'a mut User;
    type IntoIter = std::slice::IterMut<'a, User>;
    fn into_iter(self) -> Self::IntoIter {
        self.users.iter_mut()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    outlinks: BTreeSet<usize>,
}

impl User {
    pub fn add_link(&mut self, peer: usize) -> bool {
        let added = !self.outlinks.contains(&peer);
        self.outlinks.insert(peer);
        added
    }

    pub fn len(&self) -> usize {
        self.outlinks.len()
    }
}

impl<'a> IntoIterator for &'a User {
    type Item = &'a usize;
    type IntoIter = std::collections::btree_set::Iter<'a, usize>;
    fn into_iter(self) -> Self::IntoIter {
        self.outlinks.iter()
    }
}
