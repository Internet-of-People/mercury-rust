use failure::{ensure, Fallible};
use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use keyvault::{PrivateKey as KeyVaultPrivateKey, PublicKey as KeyVaultPublicKey};

pub type AsyncResult<T, E> = Box<dyn Future<Item = T, Error = E>>;
pub type AsyncFallible<T> = Box<dyn Future<Item = T, Error = failure::Error>>;

pub type ContentId = String; // Something similar to IPFS CIDv1 https://github.com/ipfs/specs/issues/130

pub type KeyId = keyvault::multicipher::MKeyId;
pub type PublicKey = keyvault::multicipher::MPublicKey;
pub type PrivateKey = keyvault::multicipher::MPrivateKey;
pub type Signature = keyvault::multicipher::MSignature;

// NOTE a.k.a DID
pub type ProfileId = KeyId;

/// Something that can sign data, but cannot give out the private key.
/// Usually implemented using a private key internally, but also enables hardware wallets.
pub trait Signer {
    fn profile_id(&self) -> &ProfileId;
    fn public_key(&self) -> PublicKey;
    fn sign(&self, data: &[u8]) -> Fallible<Signature>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedMessage {
    public_key: PublicKey,
    #[serde(with = "serde_bytes")]
    message: Vec<u8>,
    signature: Signature,
}

impl SignedMessage {
    pub fn new(public_key: PublicKey, message: Vec<u8>, signature: Signature) -> Self {
        Self { public_key, message, signature }
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    pub fn message(&self) -> &[u8] {
        &self.message
    }
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    pub fn validate(&self) -> bool {
        self.public_key.verify(&self.message, &self.signature)
    }
}

pub struct PrivateKeySigner {
    private_key: PrivateKey,
    profile_id: ProfileId,
}

impl PrivateKeySigner {
    pub fn new(private_key: PrivateKey, profile_id: ProfileId) -> Fallible<Self> {
        ensure!(
            private_key.public_key().validate_id(&profile_id),
            "Given private key and DID do not match"
        );
        Ok(Self { private_key, profile_id })
    }
}

impl Signer for PrivateKeySigner {
    fn profile_id(&self) -> &ProfileId {
        &self.profile_id
    }
    fn public_key(&self) -> PublicKey {
        self.private_key.public_key()
    }
    fn sign(&self, data: &[u8]) -> Fallible<Signature> {
        Ok(self.private_key.sign(data))
    }
}
