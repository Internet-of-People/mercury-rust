/// Size of the chain code in bytes
pub const CHAIN_CODE_SIZE: usize = 32;

/// Chain code for key derivation in Ed25519 extended private and public keys.
/// This is a 256-bit secret key that is completely independent of the private
/// key and is used as an extension to the cryptographic domain, basically an
/// extra state during iteration.
#[derive(Clone)]
pub struct ChainCode([u8; CHAIN_CODE_SIZE]);

impl ChainCode {
    /// The chain code serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> [u8; CHAIN_CODE_SIZE] {
        self.0
    }

    /// Creates a chain code from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is not [`CHAIN_CODE_SIZE`] long
    ///
    /// [`to_bytes`]: #method.to_bytes
    /// [`CHAIN_CODE_SIZE`]: ../constant.CHAIN_CODE_SIZE
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Self {
        let mut cc = [0u8; CHAIN_CODE_SIZE];
        cc.copy_from_slice(bytes.as_ref());
        Self(cc)
    }
}
