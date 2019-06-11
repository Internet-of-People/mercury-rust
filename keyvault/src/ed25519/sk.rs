use ed25519_dalek as ed;

use super::*;

/// The size of the private key in the format used by [`to_bytes`]
///
/// [`to_bytes`]: #method.to_bytes
pub const PRIVATE_KEY_SIZE: usize = ed::SECRET_KEY_LENGTH;

/// Implementation of Ed25519::PrivateKey
pub struct EdPrivateKey(ed::Keypair);

impl EdPrivateKey {
    /// The private key serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut res = Vec::with_capacity(PRIVATE_KEY_SIZE);
        res.extend_from_slice(self.0.secret.as_bytes());
        res
    }

    /// Creates a public key from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is rejected by `ed25519_dalek::SecretKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Fallible<Self> {
        let secret = ed::SecretKey::from_bytes(bytes.as_ref())?;
        let public = ed::PublicKey::from(&secret);
        let key_pair = ed::Keypair { secret, public };
        Ok(Self(key_pair))
    }
}

impl Clone for EdPrivateKey {
    fn clone(&self) -> Self {
        let secret_bytes = self.0.secret.as_bytes();
        let public_bytes = self.0.public.as_bytes();
        let secret = ed::SecretKey::from_bytes(secret_bytes).unwrap();
        let public = ed::PublicKey::from_bytes(public_bytes).unwrap();
        Self(ed::Keypair { secret, public })
    }
}

impl PrivateKey<Ed25519> for EdPrivateKey {
    fn public_key(&self) -> EdPublicKey {
        let pk = self.0.public;
        pk.into()
    }
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> EdSignature {
        let sig = self.0.sign(data.as_ref());
        sig.into()
    }
}

impl From<ed::Keypair> for EdPrivateKey {
    fn from(kp: ed::Keypair) -> Self {
        Self(kp)
    }
}
