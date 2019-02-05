//! SLIP-0010 compatible Ed25519 cryptography that allows child key derivation. There are alternative
//! Ed25519-based implementations in other projects that are incompatible with SLIP-0010, so make sure
//! this is the right derivation method for your use-case.
use failure::Fallible;

use super::{
    AsymmetricCrypto, ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PrivateKey,
    PublicKey, Seed,
};

/// This elliptic curve cryptography implements both the [AsymmetricCrypto](AsymmetricCrypto) and
/// [KeyDerivationCrypto](KeyDerivationCrypto) traits so it can be used in EcDSA, Cardano and of course,
/// Mopheus/Prometheus/Mercury.
pub struct Ed25519 {}

/// Implementation of Ed25519::KeyId
#[derive(Hash, Eq, PartialEq)]
pub struct KeyId([u8; 32]);
/// Implementation of Ed25519::PrivateKey
pub struct EdPrivateKey([u8; 32]);
/// Implementation of Ed25519::PublicKey
pub struct EdPublicKey([u8; 32]);
/// Implementation of Ed25519::Signature
pub struct Signature([u8; 32]);
/// Chain key for key derivation in Ed25519 extended private and public keys.
struct ChainKey([u8; 32]);
/// Implementation of Ed25519::ExtendedPrivateKey
pub struct EdExtPrivateKey(ChainKey, EdPrivateKey);
/// Implementation of Ed25519::ExtendedPublicKey
pub struct EdExtPublicKey(ChainKey, EdPublicKey);

macro_rules! base_encodable {
    ($name:ident : $ch:expr) => {
        impl AsRef<[u8]> for $name {
            fn as_ref(&self) -> &[u8] {
                self.0.as_ref()
            }
        }

        impl $crate::wrappers::BaseEncodable for $name {
            fn get_base_code(&self) -> char {
                $ch
            }
        }
    };
}

base_encodable!(KeyId: 'm');
base_encodable!(EdPrivateKey: 'm');
base_encodable!(EdPublicKey: 'm');
base_encodable!(Signature: 'm');

impl AsymmetricCrypto for Ed25519 {
    type KeyId = KeyId;
    type PublicKey = EdPublicKey;
    type PrivateKey = EdPrivateKey;
    type Signature = Signature;
}

impl KeyDerivationCrypto for Ed25519 {
    type ExtendedPrivateKey = EdExtPrivateKey;
    type ExtendedPublicKey = EdExtPublicKey;

    fn master(seed: &Seed) -> EdExtPrivateKey {
        unimplemented!()
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> KeyId {
        unimplemented!()
    }
    fn verify(&self, data: &[u8], sig: Signature) -> bool {
        unimplemented!()
    }
}

impl PrivateKey<Ed25519> for EdPrivateKey {
    fn public_key(&self) -> EdPublicKey {
        unimplemented!()
    }
    fn sign(&self, data: &[u8]) -> Signature {
        unimplemented!()
    }
}

impl ExtendedPrivateKey<Ed25519> for EdExtPrivateKey {
    fn derive_normal_child(&self, idx: i32) -> Fallible<EdExtPrivateKey> {
        unimplemented!()
    }
    fn derive_hardened_child(&self, idx: i32) -> Fallible<EdExtPrivateKey> {
        unimplemented!()
    }
    fn neuter(&self) -> EdExtPublicKey {
        unimplemented!()
    }
    fn as_private_key(&self) -> EdPrivateKey {
        unimplemented!()
    }
}

impl ExtendedPublicKey<Ed25519> for EdExtPublicKey {
    fn derive_normal_child(&self, idx: i32) -> Fallible<EdExtPublicKey> {
        unimplemented!()
    }
    fn as_public_key(&self) -> EdPublicKey {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::Ed25519;
    use crate::{ExtendedPrivateKey, KeyDerivationCrypto, Seed};

    fn m_263f_1_1<T: KeyDerivationCrypto>(seed: &Seed) -> T::ExtendedPrivateKey {
        let master = T::master(seed);
        let mercury = master.derive_hardened_child(0x263F).unwrap();
        let first_profile = mercury.derive_hardened_child(1).unwrap();
        let first_app_in_first_profile = first_profile.derive_hardened_child(1).unwrap();
        first_app_in_first_profile
    }

    #[should_panic(expected = "not yet implemented")]
    #[test]
    fn test_generic() {
        let seed = Seed::generate_new();
        let first_app_in_first_profile = m_263f_1_1::<Ed25519>(&seed);
    }
}
