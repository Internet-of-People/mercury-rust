//! SLIP-0010 compatible Ed25519 cryptography that allows child key derivation. There are alternative
//! Ed25519-based implementations in other projects that are incompatible with SLIP-0010, so make sure
//! this is the right derivation method for your use-case.

mod ext_sk;
mod id;
mod pk;
mod sig;
mod sk;

use hmac::Mac;

use super::*;

/// This elliptic curve cryptography implements both the [AsymmetricCrypto](AsymmetricCrypto) and
/// [KeyDerivationCrypto](KeyDerivationCrypto) traits so it can be used in EcDSA, Cardano and of course,
/// Mopheus/Prometheus/Mercury.
pub struct Ed25519 {}

pub use cc::{ChainCode, CHAIN_CODE_SIZE};
pub use ext_sk::EdExtPrivateKey;
pub use id::{EdKeyId, KEY_ID_SALT, KEY_ID_SIZE, KEY_ID_VERSION1};
pub use pk::{EdPublicKey, PUBLIC_KEY_SIZE};
pub use sig::{EdSignature, SIGNATURE_SIZE, SIGNATURE_VERSION1};
pub use sk::{EdPrivateKey, PRIVATE_KEY_SIZE};

impl AsymmetricCrypto for Ed25519 {
    type KeyId = EdKeyId;
    type PublicKey = EdPublicKey;
    type PrivateKey = EdPrivateKey;
    type Signature = EdSignature;
}

impl KeyDerivationCrypto for Ed25519 {
    type ExtendedPrivateKey = EdExtPrivateKey;
    type ExtendedPublicKey = EdPublicKey; // No need for extension, because there is no derivation

    fn master(seed: &Seed) -> EdExtPrivateKey {
        EdExtPrivateKey::cook_new(SLIP10_SEED_HASH_SALT, |hasher| {
            hasher.input(seed.as_bytes());
        })
    }
}

/// Since Wigy could not find any constant expression for the length of `u8` in bytes
/// (`std::u8::LEN` could be a good place), this is some manual trickery to define our
/// "standard version byte length in bytes"
pub const VERSION_SIZE: usize = 1;

/// SLIP-0010 defines keyed hashing for master key derivation. This does domain separation
/// for different cryptographic algorithms. This is the standard key for Ed25519
pub const SLIP10_SEED_HASH_SALT: &[u8] = b"ed25519 seed";

#[cfg(test)]
mod tests {
    use crate::ed25519::Ed25519;
    use crate::{ExtendedPrivateKey, KeyDerivationCrypto, Seed};

    fn m_mercury_1_1<T: KeyDerivationCrypto>(seed: &Seed) -> T::ExtendedPrivateKey {
        let master = T::master(seed);
        let mercury = master.derive_hardened_child(crate::BIP43_PURPOSE_MERCURY).unwrap();
        let first_profile = mercury.derive_hardened_child(1).unwrap();
        first_profile.derive_hardened_child(1).unwrap()
    }

    #[test]
    fn test_generic() {
        let seed = Seed::generate_new();
        let _first_app_in_first_profile = m_mercury_1_1::<Ed25519>(&seed);
    }

    mod key_id {
        use crate::{ed25519::EdPublicKey, PublicKey};

        fn test(pk_hex: &str, key_id_hex: &str) {
            let pk_bytes = hex::decode(pk_hex).unwrap();
            let pk = EdPublicKey::from_bytes(pk_bytes).unwrap();

            let key_id = pk.key_id();

            assert_eq!(hex::encode(&key_id.to_bytes()), key_id_hex)
        }

        #[test]
        fn case0() {
            test(
                "0000000000000000000000000000000000000000000000000000000000000000",
                "010f0fd1fbe13e7e585ee8a14a27221225",
            );
        }

        #[test]
        fn case1() {
            test(
                "8fe9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a",
                "0182d4ecfc12c5ad8efa5ef494f47e5285",
            );
        }

        #[test]
        fn case2() {
            test(
                "8ee9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a",
                "01d8245272e2317ef53b26407e925edf7e",
            );
        }
    }

    mod derivation {
        use crate::{
            ed25519::{Ed25519, EdExtPrivateKey},
            ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, Seed,
        };
        struct TestDerivation {
            xprv: EdExtPrivateKey,
        }

        impl TestDerivation {
            fn new(seed_hex: &str) -> Self {
                let seed_bytes = hex::decode(seed_hex).unwrap();
                let seed = Seed::from_bytes(&seed_bytes).unwrap();
                let master = Ed25519::master(&seed);
                Self { xprv: master }
            }

            fn assert_state(&self, chain_code_hex: &str, sk_hex: &str, pk_hex: &str) {
                let xprv = &self.xprv;

                let chain_code_bytes = xprv.chain_code().to_bytes();
                assert_eq!(hex::encode(chain_code_bytes), chain_code_hex);

                let sk = xprv.as_private_key();
                let sk_bytes = sk.to_bytes();
                assert_eq!(hex::encode(sk_bytes), sk_hex);

                let pk = xprv.neuter().as_public_key();
                let pk_bytes = pk.to_bytes();
                assert_eq!(hex::encode(pk_bytes), pk_hex);
            }

            fn derive(&mut self, idx: i32) {
                let xprv = self.xprv.derive_hardened_child(idx).unwrap();
                self.xprv = xprv;
            }
        }

        // https://github.com/satoshilabs/slips/blob/master/slip-0010.md#test-vector-2-for-ed25519
        #[test]
        fn test_slip_0010_vector2() {
            let mut t = TestDerivation::new("fffcf9f6f3f0edeae7e4e1dedbd8d5d2cfccc9c6c3c0bdbab7b4b1aeaba8a5a29f9c999693908d8a8784817e7b7875726f6c696663605d5a5754514e4b484542");
            t.assert_state(
                "ef70a74db9c3a5af931b5fe73ed8e1a53464133654fd55e7a66f8570b8e33c3b",
                "171cb88b1b3c1db25add599712e36245d75bc65a1a5c9e18d76f9f2b1eab4012",
                "8fe9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a",
            );
            t.derive(0);
            t.assert_state(
                "0b78a3226f915c082bf118f83618a618ab6dec793752624cbeb622acb562862d",
                "1559eb2bbec5790b0c65d8693e4d0875b1747f4970ae8b650486ed7470845635",
                "86fab68dcb57aa196c77c5f264f215a112c22a912c10d123b0d03c3c28ef1037",
            );
            t.derive(2_147_483_647);
            t.assert_state(
                "138f0b2551bcafeca6ff2aa88ba8ed0ed8de070841f0c4ef0165df8181eaad7f",
                "ea4f5bfe8694d8bb74b7b59404632fd5968b774ed545e810de9c32a4fb4192f4",
                "5ba3b9ac6e90e83effcd25ac4e58a1365a9e35a3d3ae5eb07b9e4d90bcf7506d",
            );
            t.derive(1);
            t.assert_state(
                "73bd9fff1cfbde33a1b846c27085f711c0fe2d66fd32e139d3ebc28e5a4a6b90",
                "3757c7577170179c7868353ada796c839135b3d30554bbb74a4b1e4a5a58505c",
                "2e66aa57069c86cc18249aecf5cb5a9cebbfd6fadeab056254763874a9352b45",
            );
            t.derive(2_147_483_646);
            t.assert_state(
                "0902fe8a29f9140480a00ef244bd183e8a13288e4412d8389d140aac1794825a",
                "5837736c89570de861ebc173b1086da4f505d4adb387c6a1b1342d5e4ac9ec72",
                "e33c0f7d81d843c572275f287498e8d408654fdf0d1e065b84e2e6f157aab09b",
            );
            t.derive(2);
            t.assert_state(
                "5d70af781f3a37b829f0d060924d5e960bdc02e85423494afc0b1a41bbe196d4",
                "551d333177df541ad876a60ea71f00447931c0a9da16f227c11ea080d7391b8d",
                "47150c75db263559a70d5778bf36abbab30fb061ad69f69ece61a72b0cfa4fc0",
            );
        }
    }

    #[test]
    fn test_private_from_bytes() {
        use super::EdPrivateKey;
        use crate::PrivateKey;

        let sk_bytes =
            hex::decode("171cb88b1b3c1db25add599712e36245d75bc65a1a5c9e18d76f9f2b1eab4012")
                .unwrap();
        let sk = EdPrivateKey::from_bytes(sk_bytes).unwrap();

        let pk = sk.public_key();
        let pk_bytes = pk.to_bytes();
        assert_eq!(
            hex::encode(pk_bytes),
            "8fe9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a"
        );
    }

    #[test]
    fn test_public_from_bytes() {
        use super::EdPublicKey;
        let pk_bytes =
            hex::decode("278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e")
                .unwrap();
        let _pk = EdPublicKey::from_bytes(&pk_bytes);
    }

    /// Test vectors based on https://tools.ietf.org/html/rfc8032#page-24
    mod sign_verify {
        use crate::ed25519::EdPrivateKey;
        use crate::{PrivateKey, PublicKey};

        fn test(sk_hex: &str, pk_hex: &str, msg_hex: &str, sig_hex: &str) {
            let sk_bytes = hex::decode(sk_hex).unwrap();
            let sk = EdPrivateKey::from_bytes(sk_bytes.as_slice()).unwrap();

            let pk = sk.public_key();
            let pk_bytes = pk.to_bytes();
            assert_eq!(hex::encode(&pk_bytes[..]), pk_hex);

            let msg = hex::decode(msg_hex.replace(' ', "")).unwrap();
            let sig = sk.sign(msg.as_slice());
            let sig_bytes = sig.to_bytes();
            assert_eq!(hex::encode(&sig_bytes[..]), sig_hex.replace(' ', ""));

            assert!(pk.verify(msg, &sig));
        }

        #[test]
        fn char_0() {
            test(
                "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
                "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
                "",
                "01e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155 \
                 5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
            );
        }

        #[test]
        fn char_1() {
            test(
                "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
                "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
                "72",
                "0192a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da \
                 085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00",
            );
        }

        #[test]
        fn char_2() {
            test(
                "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
                "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
                "af82",
                "016291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac \
                 18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a",
            );
        }

        #[test]
        fn char_1023() {
            test(
                "f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5",
                "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e",
                "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98 \
                 fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8 \
                 79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d \
                 658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc \
                 1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe \
                 ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e \
                 06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef \
                 efd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec7 \
                 aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed1 \
                 85ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb2 \
                 d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc24 \
                 554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f270 \
                 88d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbcc \
                 2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b07 \
                 07e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128ba \
                 b27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51a \
                 ddd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429e \
                 c96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb7 \
                 51fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c \
                 42f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8 \
                 ca61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34df \
                 f7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08 \
                 d78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649 \
                 de6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e4 \
                 88acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a3 \
                 2ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e \
                 6aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5f \
                 b93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b5 \
                 0d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef1 \
                 369546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380d \
                 b2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c \
                 0618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d0",
                "010aab4c900501b3e24d7cdf4663326a3a87df5e4843b2cbdb67cbf6e460fec350 \
                 aa5371b1508f9f4528ecea23c436d94b5e8fcd4f681e30a6ac00a9704a188a03",
            );
        }

        #[test]
        fn char_64() {
            test(
                "833fe62409237b9d62ec77587520911e9a759cec1d19755b7da901b96dca3d42",
                "ec172b93ad5e563bf4932c70e1245034c35467ef2efd4d64ebf819683467e2bf",
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a \
                 2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
                "01dc2a4459e7369633a52b1bf277839a00201009a3efbf3ecb69bea2186c26b589 \
                 09351fc9ac90b3ecfdfbc7c66431e0303dca179c138ac17ad9bef1177331a704",
            );
        }
    }
}
