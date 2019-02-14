use ed25519_dalek as ed;

/// Implementation of Ed25519::Signature
pub struct EdSignature(ed::Signature);

impl EdSignature {
    /// The signature serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> [u8; ed::SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }

    /// Creates a signature from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is rejected by `ed25519_dalek::SecretKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Self {
        let sig = ed::Signature::from_bytes(bytes.as_ref()).unwrap();
        EdSignature(sig)
    }
}

impl From<ed::Signature> for EdSignature {
    fn from(sig: ed::Signature) -> Self {
        EdSignature(sig)
    }
}

impl<'a> From<&'a EdSignature> for &'a ed::Signature {
    fn from(sig: &'a EdSignature) -> Self {
        &sig.0
    }
}
