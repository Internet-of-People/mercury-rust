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
        EdExtPrivateKey(ChainCode(*c), EdPrivateKey::from(sk))
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> KeyId {
        unimplemented!()
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: EdSignature) -> bool {
        let verifier = &self.0;
        signatory::ed25519::verify(verifier, data.as_ref(), &sig.0).is_ok()
    }
}

impl PrivateKey<Ed25519> for EdPrivateKey {
    fn public_key(&self) -> EdPublicKey {
        let signer = Ed25519Signer::from(&self.0);
        let pk = &signatory::ed25519::public_key(&signer).unwrap();
        EdPublicKey(pk.into())
    }
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> EdSignature {
        let signer = Ed25519Signer::from(&self.0);
        let sig = signatory::ed25519::sign(&signer, data.as_ref()).unwrap();
        EdSignature(sig)
    }
}

impl<D: AsRef<[u8]>> From<D> for EdPrivateKey {
    fn from(bytes: D) -> Self {
        EdPrivateKey(signatory::ed25519::Seed::from_bytes(bytes).unwrap())
    }
}

impl AsRef<[u8]> for EdSignature {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
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
    use crate::{PrivateKey, PublicKey, ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, Seed};

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

    // https://tools.ietf.org/html/rfc8032#page-24
    #[test]
    fn test_sign() {
        use super::{EdPrivateKey};

        let sk_bytes = hex::decode("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5").unwrap();
        let sk = EdPrivateKey::from(sk_bytes.as_slice());

        // let pk = sk.public_key();
        // let pk_bytes = pk.as_bytes();
        // assert_eq!(hex::encode(pk_bytes), "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e");

        let message_hex = "08b8b2b733424243760fe426a4b54908 \
            632110a66c2f6591eabd3345e3e4eb98 \
            fa6e264bf09efe12ee50f8f54e9f77b1 \
            e355f6c50544e23fb1433ddf73be84d8 \
            79de7c0046dc4996d9e773f4bc9efe57 \
            38829adb26c81b37c93a1b270b20329d \
            658675fc6ea534e0810a4432826bf58c \
            941efb65d57a338bbd2e26640f89ffbc \
            1a858efcb8550ee3a5e1998bd177e93a \
            7363c344fe6b199ee5d02e82d522c4fe \
            ba15452f80288a821a579116ec6dad2b \
            3b310da903401aa62100ab5d1a36553e \
            06203b33890cc9b832f79ef80560ccb9 \
            a39ce767967ed628c6ad573cb116dbef \
            efd75499da96bd68a8a97b928a8bbc10 \
            3b6621fcde2beca1231d206be6cd9ec7 \
            aff6f6c94fcd7204ed3455c68c83f4a4 \
            1da4af2b74ef5c53f1d8ac70bdcb7ed1 \
            85ce81bd84359d44254d95629e9855a9 \
            4a7c1958d1f8ada5d0532ed8a5aa3fb2 \
            d17ba70eb6248e594e1a2297acbbb39d \
            502f1a8c6eb6f1ce22b3de1a1f40cc24 \
            554119a831a9aad6079cad88425de6bd \
            e1a9187ebb6092cf67bf2b13fd65f270 \
            88d78b7e883c8759d2c4f5c65adb7553 \
            878ad575f9fad878e80a0c9ba63bcbcc \
            2732e69485bbc9c90bfbd62481d9089b \
            eccf80cfe2df16a2cf65bd92dd597b07 \
            07e0917af48bbb75fed413d238f5555a \
            7a569d80c3414a8d0859dc65a46128ba \
            b27af87a71314f318c782b23ebfe808b \
            82b0ce26401d2e22f04d83d1255dc51a \
            ddd3b75a2b1ae0784504df543af8969b \
            e3ea7082ff7fc9888c144da2af58429e \
            c96031dbcad3dad9af0dcbaaaf268cb8 \
            fcffead94f3c7ca495e056a9b47acdb7 \
            51fb73e666c6c655ade8297297d07ad1 \
            ba5e43f1bca32301651339e22904cc8c \
            42f58c30c04aafdb038dda0847dd988d \
            cda6f3bfd15c4b4c4525004aa06eeff8 \
            ca61783aacec57fb3d1f92b0fe2fd1a8 \
            5f6724517b65e614ad6808d6f6ee34df \
            f7310fdc82aebfd904b01e1dc54b2927 \
            094b2db68d6f903b68401adebf5a7e08 \
            d78ff4ef5d63653a65040cf9bfd4aca7 \
            984a74d37145986780fc0b16ac451649 \
            de6188a7dbdf191f64b5fc5e2ab47b57 \
            f7f7276cd419c17a3ca8e1b939ae49e4 \
            88acba6b965610b5480109c8b17b80e1 \
            b7b750dfc7598d5d5011fd2dcc5600a3 \
            2ef5b52a1ecc820e308aa342721aac09 \
            43bf6686b64b2579376504ccc493d97e \
            6aed3fb0f9cd71a43dd497f01f17c0e2 \
            cb3797aa2a2f256656168e6c496afc5f \
            b93246f6b1116398a346f1a641f3b041 \
            e989f7914f90cc2c7fff357876e506b5 \
            0d334ba77c225bc307ba537152f3f161 \
            0e4eafe595f6d9d90d11faa933a15ef1 \
            369546868a7f3a45a96768d40fd9d034 \
            12c091c6315cf4fde7cb68606937380d \
            b2eaaa707b4c4185c32eddcdd306705e \
            4dc1ffc872eeee475a64dfac86aba41c \
            0618983f8741c5ef68d3a101e8a3b8ca \
            c60c905c15fc910840b94c00a0b9d0";
        let message = hex::decode(message_hex.replace(' ', "")).unwrap();
        let sig = sk.sign(message.as_slice());
        assert_eq!(hex::encode(sig.as_ref()), "0aab4c900501b3e24d7cdf4663326a3a87df5e4843b2cbdb67cbf6e460fec350aa5371b1508f9f4528ecea23c436d94b5e8fcd4f681e30a6ac00a9704a188a03");
    }
 }
