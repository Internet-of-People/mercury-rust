use failure::Fallible;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::{Index, IndexMut};

use morpheus_keyvault::Seed;

#[derive(Deserialize, Serialize)]
pub struct State {
    vault_seed: Seed,
    users: Vec<User>,
}

impl State {
    pub fn new<S: AsRef<str>>(phrase: S) -> Fallible<Self> {
        let vault_seed = morpheus_keyvault::Seed::from_bip39(phrase)?;
        let users = Default::default();
        Ok(Self { vault_seed, users })
    }

    pub fn vault_seed(&self) -> &Seed {
        &self.vault_seed
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
    pub outlinks: BTreeSet<usize>,
}

impl User {
    pub fn add_link(&mut self, peer: usize) -> bool {
        let added = !self.outlinks.contains(&peer);
        self.outlinks.insert(peer);
        added
    }
}
