use super::*;

/// The serialized byte representation for the current version of the signature algorithm
pub const SIGNATURE_VERSION1: u8 = b'\x01';

/// Size of the signature is the version byte plus the actual libsecp256k1 signature size
pub const SIGNATURE_SIZE: usize = secp::util::SIGNATURE_SIZE + VERSION_SIZE;

/// Implementation of Secp256k1::Signature
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SecpSignature(pub(super) secp::Signature);

impl SecpSignature {
    /// The signature serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(SIGNATURE_SIZE);
        res.push(SIGNATURE_VERSION1);
        res.extend_from_slice(&self.0.serialize()[..]);
        res
    }

    /// Creates a signature from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is rejected by `libsecp256k1::Signature::parse`
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
        let mut array = [0u8; secp::util::SIGNATURE_SIZE];
        array.copy_from_slice(&bytes[VERSION_SIZE..]);
        let sig = secp::Signature::parse(&array);
        Ok(Self(sig))
    }
}
