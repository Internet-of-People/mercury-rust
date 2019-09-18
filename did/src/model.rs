use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use keyvault::PublicKey as KeyVaultPublicKey;

pub type AsyncResult<T, E> = Box<dyn Future<Item = T, Error = E>>;
pub type AsyncFallible<T> = Box<dyn Future<Item = T, Error = failure::Error>>;

pub type ContentId = String; // Something similar to IPFS CIDv1 https://github.com/ipfs/specs/issues/130

pub type KeyId = keyvault::multicipher::MKeyId;
pub type PublicKey = keyvault::multicipher::MPublicKey;
pub type PrivateKey = keyvault::multicipher::MPrivateKey;
pub type Signature = keyvault::multicipher::MSignature;

// NOTE a.k.a DID
pub type ProfileId = KeyId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SignedMessage {
    public_key: PublicKey,
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
