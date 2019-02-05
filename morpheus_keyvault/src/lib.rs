#![warn(missing_docs)]

//! This library provides a high-level API to be used in Morpheus as a key-vault. It wraps multiple
//! cryptographic libraries to make it easier on the integrator.

// failure 0.1.5 is still not edition 2018-compliant. #[fail] attributes do not work without this line:
#[macro_use]
extern crate failure;

pub mod bip32;
mod bip39;
pub mod ed25519;
#[cfg(test)]
mod tests;
mod wrappers;

use failure::{bail, Fallible};
use std::borrow::Borrow;

pub trait PublicKey<C: AsymmetricCrypto + ?Sized> {
    fn key_id(&self) -> C::KeyId;
    fn verify(&self, data: &[u8], sig: C::Signature) -> bool;
}

pub trait PrivateKey<C: AsymmetricCrypto + ?Sized> {
    fn public_key(&self) -> C::PublicKey;
    fn sign(&self, data: &[u8]) -> C::Signature;
}

pub trait AsymmetricCrypto {
    type KeyId: std::hash::Hash + Eq;
    type PublicKey: PublicKey<Self>;
    type PrivateKey: PrivateKey<Self>;
    type Signature;
}

/// The seed used for BIP32 derivations.
#[derive(Debug)]
pub struct Seed {
    bytes: Vec<u8>,
}

impl Seed {
    const PASSWORD: &'static str = "morpheus";
    const MNEMONIC_WORDS: usize = 24;
    const BITS: usize = 512;

    /// Creates new 512-bit seed from hardware entropy.
    /// # Panics
    /// bip39-rs v0.5.1 uses rand::os_rng that might fail on filesystem related issues. This panic will go
    /// away when we upgrade to bip39-rs v0.6.
    pub fn generate_new() -> Self {
        let bytes = bip39::generate_new(Self::PASSWORD).expect(
            "This will not fail after we upgrade to bip39-rs v0.6 that uses rand::thread_rng",
        );
        Self { bytes }
    }

    /// Creates seed from a 24-word BIP39 mnemonic
    ///
    /// # Example
    ///
    /// ```
    /// # use morpheus_keyvault::Seed;
    /// let words = [
    ///   "plastic",
    ///   "attend",
    ///   "shadow",
    ///   "hill",
    ///   "conduct",
    ///   "whip",
    ///   "staff",
    ///   "shoe",
    ///   "achieve",
    ///   "repair",
    ///   "museum",
    ///   "improve",
    ///   "below",
    ///   "inform",
    ///   "youth",
    ///   "alpha",
    ///   "above",
    ///   "limb",
    ///   "paddle",
    ///   "derive",
    ///   "spoil",
    ///   "offer",
    ///   "hospital",
    ///   "advance",];
    /// let seed_expected = "86f07ba8b38f3de2080912569a07b21ca4ae2275bc305a14ff928c7dc5407f32a1a3a26d4e2c4d9d5e434209c1db3578d94402cf313f3546344d0e4661c9f8d9";
    /// let seed_res = Seed::from_bip39(&words);
    /// assert!(seed_res.is_ok());
    /// assert_eq!(hex::encode(seed_res.unwrap().as_bytes()), seed_expected);
    /// ```
    pub fn from_bip39<T: Borrow<str>>(words: &[T]) -> Fallible<Self> {
        if words.len() != Self::MNEMONIC_WORDS {
            bail!("Only {}-word mnemonics are supported", Self::MNEMONIC_WORDS)
        }
        let bytes = bip39::from_phrase(words, Self::PASSWORD)?;
        Ok(Self { bytes })
    }

    /// Creates seed from a raw 512-bit binary seed
    ///
    /// # Example
    ///
    /// ```
    /// # use morpheus_keyvault::Seed;
    /// let bytes = "86f07ba8b38f3de2080912569a07b21ca4ae2275bc305a14ff928c7dc5407f32a1a3a26d4e2c4d9d5e434209c1db3578d94402cf313f3546344d0e4661c9f8d9";
    /// let seed_res = Seed::from_bytes(hex::decode(bytes).unwrap().as_slice());
    /// assert!(seed_res.is_ok());
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Fallible<Self> {
        if bytes.len() * 8 != Self::BITS {
            bail!("Only {}-bit seeds are supported", Self::BITS)
        }
        let bytes = bytes.to_vec();
        Ok(Self { bytes })
    }

    /// Returns the bytes of the seed
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

pub trait ExtendedPrivateKey<C: KeyDerivationCrypto + ?Sized> {
    fn derive_normal_child(&self, idx: i32) -> Fallible<C::ExtendedPrivateKey>;
    fn derive_hardened_child(&self, idx: i32) -> Fallible<C::ExtendedPrivateKey>;
    fn neuter(&self) -> C::ExtendedPublicKey;
    fn as_private_key(&self) -> C::PrivateKey;
}

pub trait ExtendedPublicKey<C: KeyDerivationCrypto + ?Sized> {
    fn derive_normal_child(&self, idx: i32) -> Fallible<C::ExtendedPublicKey>;
    fn as_public_key(&self) -> C::PublicKey;
}

pub trait KeyDerivationCrypto: AsymmetricCrypto {
    type ExtendedPrivateKey: ExtendedPrivateKey<Self>;
    type ExtendedPublicKey: ExtendedPublicKey<Self>;

    fn master(seed: &Seed) -> Self::ExtendedPrivateKey;
}
