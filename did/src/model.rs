use futures::prelude::*;

pub type AsyncResult<T, E> = Box<Future<Item = T, Error = E>>;
pub type AsyncFallible<T> = Box<Future<Item = T, Error = failure::Error>>;

pub type ContentId = String; // Something similar to IPFS CIDv1 https://github.com/ipfs/specs/issues/130

pub type KeyId = keyvault::multicipher::MKeyId;
pub type PublicKey = keyvault::multicipher::MPublicKey;
pub type PrivateKey = keyvault::multicipher::MPrivateKey;
pub type Signature = keyvault::multicipher::MSignature;

// NOTE a.k.a DID
pub type ProfileId = KeyId;
