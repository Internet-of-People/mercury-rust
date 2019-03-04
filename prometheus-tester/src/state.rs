use failure::Fallible;
use rand::SeedableRng;
use rand_xorshift::XorShiftRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::iter::FromIterator;
use std::ops::{Index, IndexMut};

#[derive(Deserialize, Serialize)]
pub struct State {
    vault_seed: morpheus_keyvault::Seed,
    rand: XorShiftRng,
    users: Vec<User>,
    steps: usize,
}

impl State {
    pub fn new<S: AsRef<str>>(phrase: S) -> Fallible<Self> {
        let vault_seed = morpheus_keyvault::Seed::from_bip39(phrase)?;
        let rand = XorShiftRng::from_seed([42u8; 16]); // TODO config
        let users = Default::default();
        let steps = Default::default();
        Ok(Self {
            vault_seed,
            rand,
            users,
            steps,
        })
    }

    pub fn vault_seed(&self) -> &morpheus_keyvault::Seed {
        &self.vault_seed
    }

    pub fn rand(&mut self) -> &mut XorShiftRng {
        &mut self.rand
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

    pub fn steps(&self) -> usize {
        self.steps
    }

    pub fn add_step(&mut self) {
        self.steps += 1;
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

    pub fn not_links(&self, count: usize) -> Vec<usize> {
        let full = BTreeSet::from_iter(0..count);
        full.difference(&self.outlinks).cloned().collect()
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
