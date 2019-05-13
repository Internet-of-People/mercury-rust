use ed25519_dalek as ed;

use super::*;

/// The serialized byte representation for the current version of the signature algorithm
/// (standard Ed25519 signature uses SHA256 internally and its output is only dependent on
/// the private key and the message. It does not add an extra random keying that it could)
pub const SIGNATURE_VERSION1: u8 = b'\x01';

/// Size of the signature is the version byte plus the actual Dalek Ed25519 signature size
pub const SIGNATURE_SIZE: usize = ed::SIGNATURE_LENGTH + VERSION_SIZE;

/// Implementation of Ed25519::Signature
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct EdSignature(ed::Signature);

impl EdSignature {
    /// The signature serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> [u8; SIGNATURE_SIZE] {
        let mut signature_bytes = [0u8; SIGNATURE_SIZE];
        signature_bytes[0] = SIGNATURE_VERSION1;
        signature_bytes[VERSION_SIZE..].copy_from_slice(&self.0.to_bytes()[..]);
        signature_bytes
    }

    /// Creates a signature from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is rejected by `ed25519_dalek::SecretKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Fallible<Self> {
        let bytes = bytes.as_ref();
        ensure!(bytes.len() == SIGNATURE_SIZE, "Signature length is not {}", SIGNATURE_SIZE);
        ensure!(
            bytes[0] == SIGNATURE_VERSION1,
            "Only identifier version {:x} is supported",
            SIGNATURE_VERSION1
        );
        let sig = ed::Signature::from_bytes(&bytes[VERSION_SIZE..])?;
        Ok(Self(sig))
    }
}

impl From<ed::Signature> for EdSignature {
    fn from(sig: ed::Signature) -> Self {
        Self(sig)
    }
}

impl<'a> From<&'a EdSignature> for &'a ed::Signature {
    fn from(sig: &'a EdSignature) -> Self {
        &sig.0
    }
}
