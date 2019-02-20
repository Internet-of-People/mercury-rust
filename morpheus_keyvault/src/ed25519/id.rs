use blake2::{
    digest::{Input, VariableOutput},
    VarBlake2b,
};
use serde_derive::{Deserialize, Serialize};

use super::*;

/// This constant is used for keyed hashing of public keys. This does not improve the security
/// of the hash algorithm, but allows for domain separation if some use-case requires a different
/// hash of the public key with the same algorithm.
pub const KEY_ID_SALT: &[u8] = b"open social graph";

/// The size of the key identifier in bytes. Since a version byte is prepended to the
/// hash result, it is not a standard size.
pub const KEY_ID_SIZE: usize = 16 + VERSION_SIZE;

/// The serialized byte representation for the current version of the hash algorithm
/// applied on the public key to obtain the key identifier
pub const KEY_ID_VERSION1: u8 = b'\x01';

/// Implementation of Ed25519::KeyId
#[derive(Clone, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct KeyId(#[serde(with = "serde_bytes")] Vec<u8>);

impl KeyId {
    /// The public key serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> [u8; KEY_ID_SIZE] {
        let mut res = [0; KEY_ID_SIZE];
        res.copy_from_slice(&self.0);
        res
    }

    /// Creates a chain code from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is not [`KEY_ID_SIZE`] long
    ///
    /// [`to_bytes`]: #method.to_bytes
    /// [`KEY_ID_SIZE`]: ../constant.KEY_ID_SIZE
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Self {
        let bytes = bytes.as_ref();
        assert_eq!(bytes.len(), KEY_ID_SIZE);
        assert_eq!(bytes[0], KEY_ID_VERSION1);
        Self(bytes.to_owned())
    }
}

impl<D: AsRef<[u8]>> From<D> for KeyId {
    /// Creates a key id from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is not the right size or not the right version.
    ///
    /// [`to_bytes`]: #method.to_bytes
    fn from(bytes: D) -> Self {
        let bytes = bytes.as_ref();
        assert_eq!(bytes.len(), KEY_ID_SIZE);
        assert_eq!(bytes[0], KEY_ID_VERSION1);
        KeyId(bytes.to_owned())
    }
}

impl From<&EdPublicKey> for KeyId {
    fn from(pk: &EdPublicKey) -> KeyId {
        let mut hasher = VarBlake2b::new_keyed(KEY_ID_SALT, KEY_ID_SIZE - VERSION_SIZE);
        hasher.input(pk.to_bytes());
        let mut hash = Vec::with_capacity(KEY_ID_SIZE);
        hash.push(KEY_ID_VERSION1);
        hasher.variable_result(|h| hash.extend_from_slice(h));
        KeyId(hash)
    }
}

impl std::str::FromStr for KeyId {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut chars = src.chars();
        ensure!(chars.next() == Some('I'), "Identifiers must start with 'I'");
        ensure!(
            chars.next() == Some('e'),
            "Only Ed25519 cipher is supported"
        );
        let (_base, binary) = multibase::decode(chars.as_str())?;
        ensure!(
            binary[0] == KEY_ID_VERSION1,
            "Only identifier version {:x} is supported",
            KEY_ID_VERSION1
        );
        ensure!(
            binary.len() == KEY_ID_SIZE,
            "Identifier length is not {}",
            KEY_ID_SIZE
        );
        Ok(KeyId(binary))
    }
}

impl From<&KeyId> for String {
    fn from(src: &KeyId) -> Self {
        let mut output = multibase::encode(multibase::Base58btc, &src.0);
        output.insert_str(0, "Ie"); // Logically 'I' and 'e' belongs to different concepts
        output
    }
}

impl std::fmt::Display for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl std::fmt::Debug for KeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (self as &std::fmt::Display).fmt(f)
    }
}