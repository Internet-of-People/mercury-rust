//! A type-erased version of [`AsymmetricCrypto`] and [`KeyDerivationCrypto`]. Serialized versions
//! of crypto concepts, like [`KeyId`], [`PublicKey`], [`PrivateKey`], [`Signature`],
//! [`ExtendedPrivateKey`] and [`ExtendedPublicKey`] can be all deserialized into
//! their [`MultiCipher`] versions.
//! This allows multiple cryptographic algorithms to co-exist in a software, which is needed
//! during migration of a single software to a new cryptography, or which is the status quo in
//! larger software ecosystems.
//!
//! [`MultiCipher`] can be thought of a variant of multiple incompatible cipher suits, which are
//! strongly typed, but are chosen at run-time.
//!
//! [`MultiCipher`]: struct.MultiCipher.html
//! [`AsymmetricCrypto`]: ../trait.AsymmetricCrypto.html
//! [`KeyDerivationCrypto`]: ../trait.KeyDerivationCrypto.html
//! [`KeyId`]: ../trait.KeyId.html
//! [`PublicKey`]: ../trait.PublicKey.html
//! [`PrivateKey`]: ../trait.PrivateKey.html
//! [`Signature`]: ../trait.Signature.html
//! [`ExtendedPrivateKey`]: ../trait.ExtendedPrivateKey.html
//! [`ExtendedPublicKey`]: ../trait.ExtendedPublicKey.html
use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;

use crate::ed25519::{self, EdPrivateKey, EdPublicKey, EdSignature};
use crate::{AsymmetricCrypto, PrivateKey, PublicKey};

/// A discriminator type that is used to keep the type-safety of the erased types in [`multicipher`]
///
/// [`multicipher`]: index.html
#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum CipherSuite {
    /// The object tagged with this variant belongs to the [`ed25519`] module
    ///
    /// [`ed25519`]: ../ed25519/index.html
    Ed25519,
    /// Well, we only have a single suite implemented yet, so this is a distinct suite that
    /// uses the same code as the [`Ed25519`] variant does
    ///
    /// [`Ed25519`]: #variant.Ed25519
    TotallyNotEd25519,
}

macro_rules! e {
    (variant) => {
        CipherSuite::Ed25519
    };
    (id) => {
        ed25519::KeyId
    };
    (pk) => {
        EdPublicKey
    };
    (sk) => {
        EdPrivateKey
    };
    (sig) => {
        EdSignature
    };
}

macro_rules! f {
    (variant) => {
        CipherSuite::TotallyNotEd25519
    };
    (id) => {
        ed25519::KeyId
    };
    (pk) => {
        EdPublicKey
    };
    (sk) => {
        EdPrivateKey
    };
    (sig) => {
        EdSignature
    };
}

macro_rules! reify {
    ($suite:ident, $type:tt, $x:expr) => {{
        assert!($x.suite == $suite!(variant));
        $x.erased.downcast_ref::<$suite!($type)>().unwrap()
    }};
}

macro_rules! erase {
    ($suite:ident, $type:ident, $result:expr) => {
        $type {
            suite: $suite!(variant),
            erased: Box::new($result) as Box<Any>,
        }
    };
}

macro_rules! multi {
    ($callback:ident($self_:tt)) => {
        multi!($callback($self_,))
    };
    ($callback:ident($self_:tt, $($args:tt)*) ) => {
        match $self_.suite {
            e!(variant) => multi!(@case e $callback $self_ [ $($args),* ]),
            f!(variant) => multi!(@case f $callback $self_ [ $($args),* ]),
        }
    };
    (@case $suite:ident $callback:ident $self_:tt [ ]) => {
        $callback!($suite, $self_)
    };
    (@case $suite:ident $callback:ident $self_:tt [ $($args:tt),* ]) => {
        $callback!($suite, $self_, $($args)*)
    };
}

/// See the [module-level description](index.html).
pub struct MultiCipher {}

#[allow(clippy::new_without_default_derive)]
impl MultiCipher {
    /// Creates a new instance, combining all ciphers that were implemented at compile-time of this
    /// crate.
    pub fn new() -> Self {
        Self {}
    }
}

impl AsymmetricCrypto for MultiCipher {
    type KeyId = MKeyId;
    type PublicKey = MPublicKey;
    type PrivateKey = MPrivateKey;
    type Signature = MSignature;
}

/// Type-erased key id
#[derive(Debug)]
pub struct MKeyId {
    suite: CipherSuite,
    erased: Box<Any>,
}

macro_rules! mkeyid_eq {
    ($suite:ident, $self_:tt, $other:ident) => {
        reify!($suite, id, $self_).eq(reify!($suite, id, $other))
    };
}

impl PartialEq<MKeyId> for MKeyId {
    fn eq(&self, other: &Self) -> bool {
        if self.suite != other.suite {
            return false;
        }
        multi!(mkeyid_eq(self, other))
    }
}

impl Eq for MKeyId {}

macro_rules! mkeyid_hash {
    ($suite:ident, $self_:tt, $state:expr) => {
        reify!($suite, id, $self_).hash($state)
    };
}

impl Hash for MKeyId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.suite.hash(state);
        multi!(mkeyid_hash(self, state));
    }
}

/// Type-erased public key
pub struct MPublicKey {
    suite: CipherSuite,
    erased: Box<Any>,
}

macro_rules! pk_key_id {
    ($suite:ident, $self_:tt) => {{
        let result = reify!($suite, pk, $self_).key_id();
        erase!($suite, MKeyId, result)
    }};
}

macro_rules! pk_verify {
    ($suite:ident, $self_:tt, $data:ident, $sig:ident) => {
        reify!($suite, pk, $self_).verify($data, reify!($suite, sig, $sig))
    };
}

impl PublicKey<MultiCipher> for MPublicKey {
    fn key_id(&self) -> MKeyId {
        multi!(pk_key_id(self))
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &MSignature) -> bool {
        if self.suite != sig.suite {
            return false;
        }
        multi!(pk_verify(self, data, sig))
    }
}

/// Type-erased private key
pub struct MPrivateKey {
    suite: CipherSuite,
    erased: Box<Any>,
}

macro_rules! sk_public_key {
    ($suite:ident, $self_:tt) => {{
        let result = reify!($suite, sk, $self_).public_key();
        erase!($suite, MPublicKey, result)
    }};
}

macro_rules! sk_sign {
    ($suite:ident, $self_:tt, $data:ident) => {{
        let result = reify!($suite, sk, $self_).sign($data);
        erase!($suite, MSignature, result)
    }};
}

impl PrivateKey<MultiCipher> for MPrivateKey {
    fn public_key(&self) -> MPublicKey {
        multi!(sk_public_key(self))
    }
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> MSignature {
        multi!(sk_sign(self, data))
    }
}

/// Type-erased signature
pub struct MSignature {
    suite: CipherSuite,
    erased: Box<Any>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() {
        let _cipher = MultiCipher::new();
    }

}
