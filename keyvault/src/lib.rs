#![warn(missing_docs)]

//! This library provides a high-level API to be used in Morpheus as a key-vault. It wraps multiple
//! cryptographic libraries to make it easier on the integrator.

// failure 0.1.5 is still not edition 2018-compliant. #[fail] attributes do not work without this line:
#[macro_use]
extern crate failure;

pub mod bip32;
mod bip39;
mod cc;
pub mod ed25519;
pub mod multicipher;
pub mod secp256k1;
#[cfg(test)]
mod tests;

use failure::{bail, Fallible};
use serde::{Deserialize, Serialize};

pub use crate::bip39::Bip39ErrorKind;
pub use hmac::Mac;

/// A public key (also called shared key or pk in some literature) is that part of an asymmetric keypair
/// which can be used to verify the authenticity of the sender of a message or to encrypt a message that
/// can only be decrypted by a single recipient. In both cases this other party owns the [`PrivateKey`]
/// part of the keypair and never shares it with anyone else.
///
/// [`PrivateKey`]: trait.PrivateKey.html
pub trait PublicKey<C: AsymmetricCrypto + ?Sized> {
    /// Calculates the ID (also called fingerprint or address in some literature) of the public key. In
    /// some algorithms the public key is only revealed in point-to-point communications and a keypair is
    /// identified only by the digest of the public key in all other channels.
    fn key_id(&self) -> C::KeyId;

    // TODO This will probably be needed: consider
    //      - timing attack for validation with key_id() generation
    //      - KeyId might have an older version which makes key_id() unusable for verification
    // fn validate_id(&self, id: &C::KeyId) -> bool;

    /// This method can be used to verify if a given signature for a message was made using the private
    /// key that belongs to this public key. See also [`PrivateKey::sign`]
    ///
    /// [`PrivateKey::sign`]: trait.PrivateKey.html#tymethod.sign
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &C::Signature) -> bool;
}

/// A private key (also called secret key or sk in some literature) is the part of an asymmetric keypair
/// which is never shared with anyone. It is used to sign a message sent to any recipient or to decrypt a
/// message that was sent encrypted from any recipients.
pub trait PrivateKey<C: AsymmetricCrypto + ?Sized> {
    /// Calculates the [`PublicKey`] that belongs to this private key. These two keys together form an
    /// asymmetric keypair, where the private key cannot be calculated from the public key with a reasonable
    /// effort, but the public key can be calculated from the private key cheaply.
    ///
    /// [`PublicKey`]: trait.PublicKey.html
    fn public_key(&self) -> C::PublicKey;

    /// Calculates the signature of a message that can be then verified using [`PublicKey::verify`]
    ///
    /// [`PublicKey::verify`]: trait.PublicKey.html#tymethod.verify
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> C::Signature;
}

/// An implementation of this trait defines a family of types that fit together perfectly to form a
/// cryptography using asymmetric keypairs.
pub trait AsymmetricCrypto {
    /// The ID (also called fingerprint or address in some literature) of the public key. See
    /// [`PublicKey::key_id`] for more details.
    ///
    /// [`PublicKey::key_id`]: trait.PublicKey.html#tymethod.key_id
    type KeyId: std::hash::Hash + Eq;

    /// See [`PublicKey`] for more details.
    ///
    /// [`PublicKey`]: trait.PublicKey.html
    type PublicKey: PublicKey<Self>;

    /// See [`PrivateKey`] for more details.
    ///
    /// [`PrivateKey`]: trait.PrivateKey.html
    type PrivateKey: PrivateKey<Self>;

    /// The signature of a given message with a given private key. Its size and representation is up
    /// to the implementation.
    type Signature;
}

// TODO consider de/serialize attributes here, currently only needed for demo
/// The seed used for BIP32 derivations.
#[derive(Debug, Deserialize, Serialize)]
pub struct Seed {
    bytes: Vec<u8>,
}

impl Seed {
    const PASSWORD: &'static str = "morpheus";
    const MNEMONIC_WORDS: usize = 24;
    const BITS: usize = 512;

    /// Creates a new
    pub fn generate_bip39() -> String {
        bip39::generate_new_phrase(Self::MNEMONIC_WORDS)
    }

    /// Creates new 512-bit seed from hardware entropy.
    /// # Panics
    /// bip39-rs v0.5.1 uses rand::os_rng that might fail on filesystem related issues. This panic will go
    /// away when we upgrade to bip39-rs v0.6.
    pub fn generate_new() -> Self {
        let bytes = bip39::generate_new(Self::PASSWORD);
        Self { bytes }
    }

    /// Creates seed from a 24-word BIP39 mnemonic
    ///
    /// # Example
    ///
    /// ```
    /// # use keyvault::Seed;
    /// let phrase = "plastic attend shadow hill conduct whip staff shoe achieve repair museum improve below inform youth alpha above limb paddle derive spoil offer hospital advance";
    /// let seed_expected = "86f07ba8b38f3de2080912569a07b21ca4ae2275bc305a14ff928c7dc5407f32a1a3a26d4e2c4d9d5e434209c1db3578d94402cf313f3546344d0e4661c9f8d9";
    /// let seed_res = Seed::from_bip39(phrase);
    /// assert!(seed_res.is_ok());
    /// assert_eq!(hex::encode(seed_res.unwrap().as_bytes()), seed_expected);
    /// ```
    pub fn from_bip39<S: AsRef<str>>(phrase: S) -> Fallible<Self> {
        if phrase.as_ref().split(' ').count() != Self::MNEMONIC_WORDS {
            bail!("Only {}-word mnemonics are supported", Self::MNEMONIC_WORDS)
        }
        let bytes = bip39::from_phrase(phrase, Self::PASSWORD)?;
        Ok(Self { bytes })
    }

    /// Checks if a word is present in the BIP39 dictionary
    ///
    /// # Example
    ///
    /// ```
    /// # use keyvault::Seed;
    /// assert!(Seed::check_word("abandon"));
    /// assert!(!Seed::check_word("Abandon"));
    /// assert!(!Seed::check_word("avalon"));
    /// ```
    pub fn check_word(word: &str) -> bool {
        bip39::check_word(word)
    }

    /// Creates seed from a raw 512-bit binary seed
    ///
    /// # Example
    ///
    /// ```
    /// # use keyvault::Seed;
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

    // TODO this should be changed to something like Entropy::unlock(password) -> Seed
    /// Returns the bytes of the seed
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

/// The hashing algorithm used for deriving child keys in SLIP-0010
pub type HmacSha512 = hmac::Hmac<sha2::Sha512>;

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

    // TODO a password argument should be added here
    fn master(seed: &Seed) -> Self::ExtendedPrivateKey;
}

/// Unicode code point for planet mercury
pub const BIP43_PURPOSE_MERCURY: i32 = 0x263F;

/// Unicode code point for sleeping symbol
pub const BIP43_PURPOSE_MORPHEUS: i32 = 0x1F4A4;
