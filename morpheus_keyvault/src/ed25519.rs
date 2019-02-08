//! SLIP-0010 compatible Ed25519 cryptography that allows child key derivation. There are alternative
//! Ed25519-based implementations in other projects that are incompatible with SLIP-0010, so make sure
//! this is the right derivation method for your use-case.
use ed25519_dalek as ed;
use failure::Fallible;
use hmac::Mac;

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
pub struct KeyId([u8; KEY_ID_SIZE]);

/// Implementation of Ed25519::PrivateKey
pub struct EdPrivateKey(ed::Keypair);

/// Implementation of Ed25519::PublicKey
pub struct EdPublicKey(ed::PublicKey);

/// Implementation of Ed25519::Signature
pub struct EdSignature(ed::Signature);

/// Chain key for key derivation in Ed25519 extended private and public keys.
#[derive(Clone)]
struct ChainCode([u8; CHAIN_CODE_SIZE]);

/// Implementation of Ed25519::ExtendedPrivateKey
pub struct EdExtPrivateKey {
    chain_code: ChainCode,
    sk: EdPrivateKey,
}

/// Implementation of Ed25519::ExtendedPublicKey
pub struct EdExtPublicKey {
    #[allow(dead_code)]
    chain_code: ChainCode,
    pk: EdPublicKey,
}

impl AsymmetricCrypto for Ed25519 {
    type KeyId = KeyId;
    type PublicKey = EdPublicKey;
    type PrivateKey = EdPrivateKey;
    type Signature = EdSignature;
}

const KEY_ID_SIZE: usize = 32;
const CHAIN_CODE_SIZE: usize = 32;
const SLIP10_SEED_HASH_SALT: &[u8] = b"ed25519 seed";
type HmacSha512 = hmac::Hmac<sha2::Sha512>;

impl KeyDerivationCrypto for Ed25519 {
    type ExtendedPrivateKey = EdExtPrivateKey;
    type ExtendedPublicKey = EdExtPublicKey;

    fn master(seed: &Seed) -> EdExtPrivateKey {
        let mut hasher = HmacSha512::new_varkey(SLIP10_SEED_HASH_SALT).unwrap();

        hasher.input(seed.as_bytes());

        let hash_arr = hasher.result().code();
        let hash_bytes = hash_arr.as_slice();

        let sk_bytes = &hash_bytes[..ed::SECRET_KEY_LENGTH];
        let mut c_bytes: [u8; CHAIN_CODE_SIZE] = Default::default();
        c_bytes.copy_from_slice(&hash_bytes[ed::SECRET_KEY_LENGTH..]);

        let chain_code = ChainCode(c_bytes);
        let sk = EdPrivateKey::from(sk_bytes);
        EdExtPrivateKey { chain_code, sk }
    }
}

impl EdPublicKey {
    /// The public key serialized in a format that can be fed to [`from::<AsRef<[u8]>>`]
    ///
    /// [`from::<AsRef<[u8]>>`]: #impl-From<D>
    pub fn to_bytes(&self) -> [u8; ed::PUBLIC_KEY_LENGTH] {
        self.0.to_bytes()
    }
}

impl Clone for EdPublicKey {
    fn clone(&self) -> Self {
        let public_bytes = self.0.as_bytes();
        let public = ed::PublicKey::from_bytes(public_bytes).unwrap();
        Self(public)
    }
}

impl<D: AsRef<[u8]>> From<D> for EdPublicKey {
    /// Creates a public key from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is rejected by `ed25519_dalek::PublicKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    fn from(bytes: D) -> Self {
        let pk = ed::PublicKey::from_bytes(bytes.as_ref()).unwrap();
        EdPublicKey(pk)
    }
}

impl PublicKey<Ed25519> for EdPublicKey {
    fn key_id(&self) -> KeyId {
        unimplemented!()
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: EdSignature) -> bool {
        let res = self.0.verify(data.as_ref(), &sig.0);
        res.is_ok()
    }
}

impl EdPrivateKey {
    /// The private key serialized in a format that can be fed to [`from::<AsRef<[u8]>>`]
    ///
    /// [`from::<AsRef<[u8]>>`]: #impl-From<D>
    pub fn to_bytes(&self) -> [u8; ed::SECRET_KEY_LENGTH] {
        self.0.secret.to_bytes()
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

impl<D: AsRef<[u8]>> From<D> for EdPrivateKey {
    /// Creates a public key from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is rejected by `ed25519_dalek::SecretKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    fn from(bytes: D) -> Self {
        let secret = ed::SecretKey::from_bytes(bytes.as_ref()).unwrap();
        let public = ed::PublicKey::from(&secret);
        let key_pair = ed::Keypair { secret, public };
        EdPrivateKey(key_pair)
    }
}

impl PrivateKey<Ed25519> for EdPrivateKey {
    fn public_key(&self) -> EdPublicKey {
        let pk = self.0.public;
        EdPublicKey(pk)
    }
    fn sign<D: AsRef<[u8]>>(&self, data: D) -> EdSignature {
        let sig = self.0.sign(data.as_ref());
        EdSignature(sig)
    }
}

impl ChainCode {
    /// The chain code serialized.
    pub fn to_bytes(&self) -> [u8; CHAIN_CODE_SIZE] {
        self.0
    }
}

impl EdSignature {
    /// The signature serialized in a format that can be fed to [`from::<AsRef<[u8]>>`]
    ///
    /// [`from::<AsRef<[u8]>>`]: #impl-From<D>
    pub fn to_bytes(&self) -> [u8; ed::SIGNATURE_LENGTH] {
        self.0.to_bytes()
    }
}

impl<D: AsRef<[u8]>> From<D> for EdSignature {
    /// Creates a signature from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Panics
    /// If `bytes` is rejected by `ed25519_dalek::SecretKey::from_bytes`
    ///
    /// [`to_bytes`]: #method.to_bytes
    fn from(bytes: D) -> Self {
        let sig = ed::Signature::from_bytes(bytes.as_ref()).unwrap();
        EdSignature(sig)
    }
}

impl ExtendedPrivateKey<Ed25519> for EdExtPrivateKey {
    fn derive_normal_child(&self, _idx: i32) -> Fallible<EdExtPrivateKey> {
        bail!("Normal derivation of Ed25519 is invalid based on SLIP-0010.")
    }
    fn derive_hardened_child(&self, idx: i32) -> Fallible<EdExtPrivateKey> {
        ensure!(idx >= 0, "Derivation index cannot be negative");
        let idx = unsafe { std::mem::transmute::<i32, u32>(idx) };

        let mut hasher = HmacSha512::new_varkey(&self.chain_code.to_bytes()).unwrap();

        hasher.input(&[0x00u8]);
        hasher.input(&self.sk.to_bytes());
        hasher.input(&(0x8000_0000u32 + idx).to_be_bytes());

        let hash_arr = hasher.result().code();
        let hash_bytes = hash_arr.as_slice();

        let sk_bytes = &hash_bytes[..ed::SECRET_KEY_LENGTH];
        let mut c_bytes: [u8; CHAIN_CODE_SIZE] = Default::default();
        c_bytes.copy_from_slice(&hash_bytes[ed::SECRET_KEY_LENGTH..]);

        let chain_code = ChainCode(c_bytes);
        let sk = EdPrivateKey::from(sk_bytes);

        let xprv = EdExtPrivateKey { chain_code, sk };

        Ok(xprv)
    }
    fn neuter(&self) -> EdExtPublicKey {
        let chain_code = self.chain_code.clone();
        let pk = self.sk.public_key();
        EdExtPublicKey { chain_code, pk }
    }
    fn as_private_key(&self) -> EdPrivateKey {
        self.sk.clone()
    }
}

impl ExtendedPublicKey<Ed25519> for EdExtPublicKey {
    fn derive_normal_child(&self, _idx: i32) -> Fallible<EdExtPublicKey> {
        bail!("Normal derivation of Ed25519 is invalid based on SLIP-0010.")
    }
    fn as_public_key(&self) -> EdPublicKey {
        self.pk.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::Ed25519;
    use crate::{ExtendedPrivateKey, KeyDerivationCrypto, PrivateKey, PublicKey, Seed};

    fn m_263f_1_1<T: KeyDerivationCrypto>(seed: &Seed) -> T::ExtendedPrivateKey {
        let master = T::master(seed);
        let mercury = master.derive_hardened_child(0x263F).unwrap();
        let first_profile = mercury.derive_hardened_child(1).unwrap();
        first_profile.derive_hardened_child(1).unwrap()
    }

    #[test]
    fn test_generic() {
        let seed = Seed::generate_new();
        let _first_app_in_first_profile = m_263f_1_1::<Ed25519>(&seed);
    }

    // https://github.com/satoshilabs/slips/blob/master/slip-0010.md#test-vector-2-for-ed25519
    #[test]
    fn test_master() {
        let bytes = hex::decode("fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542").unwrap();
        let seed = Seed::from_bytes(&bytes).unwrap();
        let master = Ed25519::master(&seed);
        let chain_code = master.chain_code.to_bytes();
        let sk = master.as_private_key();
        let sk_bytes = sk.to_bytes();
        assert_eq!(
            hex::encode(&chain_code),
            "ef70a74db9c3a5af931b5fe73ed8e1a53464133654fd55e7a66f8570b8e33c3b"
        );
        assert_eq!(
            hex::encode(&sk_bytes),
            "171cb88b1b3c1db25add599712e36245d75bc65a1a5c9e18d76f9f2b1eab4012"
        );
        // let pk = master.neuter().as_public_key();
        // let pk_bytes = pk.0.as_bytes();
        // assert_eq!(
        //     hex::encode(pk_bytes),
        //     "00a4b2856bfec510abab89753fac1ac0e1112364e7d250545963f135f2a33188ed"
        // );
    }

    #[test]
    fn test_public_from_bytes() {
        use super::EdPublicKey;
        let pk_bytes =
            hex::decode("278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e")
                .unwrap();
        let _pk = EdPublicKey::from(&pk_bytes);
    }

    // https://tools.ietf.org/html/rfc8032#page-24
    #[test]
    fn test_sign_verify() {
        use super::EdPrivateKey;

        let sk_bytes =
            hex::decode("f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5")
                .unwrap();
        let sk = EdPrivateKey::from(sk_bytes.as_slice());

        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();
        assert_eq!(
            hex::encode(&pk_bytes[..]),
            "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e"
        );

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
        let sig_bytes = sig.to_bytes();
        assert_eq!(hex::encode(&sig_bytes[..]), "0aab4c900501b3e24d7cdf4663326a3a87df5e4843b2cbdb67cbf6e460fec350aa5371b1508f9f4528ecea23c436d94b5e8fcd4f681e30a6ac00a9704a188a03");

        assert!(pk.verify(message, sig));
    }

    #[test]
    fn test_sign_verify_2() {
        use super::EdPrivateKey;

        let sk_bytes =
            hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap();
        let sk = EdPrivateKey::from(sk_bytes.as_slice());

        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();
        assert_eq!(
            hex::encode(&pk_bytes[..]),
            "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
        );

        let message_hex = "";
        let message = hex::decode(message_hex.replace(' ', "")).unwrap();
        let sig = sk.sign(message.as_slice());
        let sig_bytes = sig.to_bytes();
        assert_eq!(hex::encode(&sig_bytes[..]), "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b");

        assert!(pk.verify(message, sig));
    }
}
