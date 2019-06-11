use blake2::VarBlake2b;
use digest::{Input, VariableOutput};

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
#[derive(Clone, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct EdKeyId(Vec<u8>);

impl EdKeyId {
    /// The key id serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    /// Creates a key id from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is not [`KEY_ID_SIZE`] long
    ///
    /// [`to_bytes`]: #method.to_bytes
    /// [`KEY_ID_SIZE`]: ../constant.KEY_ID_SIZE
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Fallible<Self> {
        let bytes = bytes.as_ref();
        ensure!(bytes.len() == KEY_ID_SIZE, "Identifier length is not {}", KEY_ID_SIZE);
        ensure!(
            bytes[0] == KEY_ID_VERSION1,
            "Only identifier version {:x} is supported",
            KEY_ID_VERSION1
        );
        Ok(Self(bytes.to_owned()))
    }
}

impl From<&EdPublicKey> for EdKeyId {
    fn from(pk: &EdPublicKey) -> EdKeyId {
        let mut hasher = VarBlake2b::new_keyed(KEY_ID_SALT, KEY_ID_SIZE - VERSION_SIZE);
        hasher.input(pk.to_bytes());
        let mut hash = Vec::with_capacity(KEY_ID_SIZE);
        hash.push(KEY_ID_VERSION1);
        hasher.variable_result(|h| hash.extend_from_slice(h));
        EdKeyId(hash)
    }
}
