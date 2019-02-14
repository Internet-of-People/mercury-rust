use ed25519_dalek as ed;

use super::{Ed25519, EdSignature, KeyId};
use crate::*;

/// Implementation of Ed25519::PublicKey
pub struct EdPublicKey(ed::PublicKey);

impl EdPublicKey {
    /// The public key serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> [u8; ed::PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }

    /// Creates a public key from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is rejected by `ed25519_dalek::PublicKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Self {
        let pk = ed::PublicKey::from_bytes(bytes.as_ref()).unwrap();
        Self(pk)
    }
}

impl Clone for EdPublicKey {
    fn clone(&self) -> Self {
        let public_bytes = self.0.as_bytes();
        let public = ed::PublicKey::from_bytes(public_bytes).unwrap();
        Self(public)
    }
}

impl From<ed::PublicKey> for EdPublicKey {
    fn from(pk: ed::PublicKey) -> Self {
        Self(pk)
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> KeyId {
        KeyId::from(self)
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &EdSignature) -> bool {
        let res = self.0.verify(data.as_ref(), sig.into());
        res.is_ok()
    }
}
