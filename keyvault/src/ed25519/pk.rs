use ed25519_dalek as ed;

use super::{Ed25519, EdKeyId, EdSignature};
use crate::*;

/// The size of the public key in the compressed format used by [`to_bytes`]
///
/// [`to_bytes`]: #method.to_bytes
pub const PUBLIC_KEY_SIZE: usize = ed::PUBLIC_KEY_LENGTH;

/// Implementation of Ed25519::PublicKey
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct EdPublicKey(ed::PublicKey);

impl EdPublicKey {
    /// The public key serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(PUBLIC_KEY_SIZE);
        res.extend_from_slice(self.0.as_bytes());
        res
    }

    /// Creates a public key from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is rejected by `ed25519_dalek::PublicKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Fallible<Self> {
        let pk = ed::PublicKey::from_bytes(bytes.as_ref())?;
        Ok(Self(pk))
    }
}

impl From<ed::PublicKey> for EdPublicKey {
    fn from(pk: ed::PublicKey) -> Self {
        Self(pk)
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> EdKeyId {
        EdKeyId::from(self)
    }
    /// We should never assume that there is only 1 public key that can verify a given
    /// signature. Actually, there are 8 public keys.
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &EdSignature) -> bool {
        let res = self.0.verify(data.as_ref(), sig.into());
        res.is_ok()
    }
}

impl ExtendedPublicKey<Ed25519> for EdPublicKey {
    fn derive_normal_child(&self, _idx: i32) -> Fallible<EdPublicKey> {
        bail!("Normal derivation of Ed25519 is invalid based on SLIP-0010.")
    }
    fn as_public_key(&self) -> EdPublicKey {
        *self
    }
}
