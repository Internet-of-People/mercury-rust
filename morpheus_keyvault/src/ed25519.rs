//! SLIP-0010 compatible Ed25519 cryptography that allows child key derivation. There are alternative
//! Ed25519-based implementations in other projects that are incompatible with SLIP-0010, so make sure
//! this is the right derivation method for your use-case.
use digest::generic_array::GenericArray;
use digest::generic_array::typenum::{U32, U64};
use failure::Fallible;
use hmac::{Hmac, Mac};
use sha2::Sha512;
use signatory_dalek::{Ed25519Signer, Ed25519Verifier};

use super::{
    AsymmetricCrypto, ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PrivateKey,
    PublicKey, Seed,
};

/// This elliptic curve cryptography implements both the [AsymmetricCrypto](AsymmetricCrypto) and
/// [KeyDerivationCrypto](KeyDerivationCrypto) traits so it can be used in EcDSA, Cardano and of course,
/// Mopheus/Prometheus/Mercury.
pub struct Ed25519 {}

/// Implementation of Ed25519::KeyId
#[derive(Clone, Hash, Eq, PartialEq)]
pub struct KeyId([u8; 32]);
/// Implementation of Ed25519::PrivateKey
#[derive(Clone)]
pub struct EdPrivateKey(signatory::ed25519::Seed);
/// Implementation of Ed25519::PublicKey
pub struct EdPublicKey(Ed25519Verifier);
/// Implementation of Ed25519::Signature
pub struct EdSignature(signatory::ed25519::Signature);
/// Chain key for key derivation in Ed25519 extended private and public keys.
struct ChainCode(GenericArray<u8, U32>);
/// Implementation of Ed25519::ExtendedPrivateKey
pub struct EdExtPrivateKey(ChainCode, EdPrivateKey);
/// Implementation of Ed25519::ExtendedPublicKey
pub struct EdExtPublicKey(ChainCode, EdPublicKey);

impl AsymmetricCrypto for Ed25519 {
    type KeyId = KeyId;
    type PublicKey = EdPublicKey;
    type PrivateKey = EdPrivateKey;
    type Signature = EdSignature;
}

const SLIP10_SEED_HASH_SALT: &[u8] = b"ed25519 seed";

impl KeyDerivationCrypto for Ed25519 {
    type ExtendedPrivateKey = EdExtPrivateKey;
    type ExtendedPublicKey = EdExtPublicKey;

    fn master(seed: &Seed) -> EdExtPrivateKey {
        let mut hasher = Hmac::<Sha512>::new_varkey(SLIP10_SEED_HASH_SALT).unwrap();
        hasher.input(seed.as_bytes());
        let hash = hasher.result().code();
        let r = hash.as_slice();
        let sk = &r[0..32];
        let c = GenericArray::<u8, U32>::from_slice(&r[32..64]);
        EdExtPrivateKey(ChainCode(*c), EdPrivateKey(signatory::ed25519::Seed::from_bytes(sk).unwrap()))
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> KeyId {
        unimplemented!()
    }
    fn verify(&self, data: &[u8], sig: EdSignature) -> bool {
        let verifier = &self.0;
        signatory::ed25519::verify(verifier, data, &sig.0).is_ok()
    }
}

impl PrivateKey<Ed25519> for EdPrivateKey {
    fn public_key(&self) -> EdPublicKey {
        let signer = Ed25519Signer::from(&self.0);
        let pk = &signatory::ed25519::public_key(&signer).unwrap();
        EdPublicKey(pk.into())
    }
    fn sign(&self, data: &[u8]) -> EdSignature {
        let signer = Ed25519Signer::from(&self.0);
        let sig = signatory::ed25519::sign(&signer, data).unwrap();
        EdSignature(sig)
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
        self.1.clone()
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
    use crate::{ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, Seed};

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

    // https://github.com/satoshilabs/slips/blob/master/slip-0010.md#test-vector-2-for-ed25519
    #[test]
    fn test_master() {
        let bytes = hex::decode("fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542").unwrap();
        let seed = Seed::from_bytes(&bytes).unwrap();
        let master = Ed25519::master(&seed);
        let chain_code = (master.0).0;
        let sk = master.as_private_key();
        let sk_bytes = (sk.0).as_secret_slice();
        assert_eq!(hex::encode(chain_code), "ef70a74db9c3a5af931b5fe73ed8e1a53464133654fd55e7a66f8570b8e33c3b");
        assert_eq!(hex::encode(sk_bytes), "171cb88b1b3c1db25add599712e36245d75bc65a1a5c9e18d76f9f2b1eab4012");
        // let pk = master.neuter().as_public_key().0.as_bytes();
        // assert_eq!(hex::encode(pk), "00a4b2856bfec510abab89753fac1ac0e1112364e7d250545963f135f2a33188ed");
    }
 }
