use super::*;

pub const XPRV_DATA_SIZE: usize = 78;
pub const SK_PREFIX: u8 = 0u8;

/// Implementation of Secp256k1::ExtendedPrivateKey
pub struct SecpExtPrivateKey {
    depth: u8,
    parent_fingerprint: Vec<u8>,
    idx: u32,
    chain_code: ChainCode,
    sk: SecpPrivateKey,
}

impl SecpExtPrivateKey {
    /// #Panics
    /// If the resulting private key is 0, we should have looped to fix that, but this implementation
    /// just panics then
    pub(crate) fn from_seed(seed: &[u8]) -> Self {
        // This unwrap would only panic if the digest algorithm had some inconsistent
        // generic parameters, but the SHA512 we use is consistent with itself
        let mut hasher = HmacSha512::new_varkey(SLIP10_SEED_HASH_SALT).unwrap();
        hasher.input(seed);
        let hash_arr = hasher.result().code();
        let hash_bytes = hash_arr.as_slice();

        let sk_bytes = &hash_bytes[..PRIVATE_KEY_SIZE];
        let cc_bytes = &hash_bytes[PRIVATE_KEY_SIZE..];

        let depth = 0;
        let parent_fingerprint = b"\x00\x00\x00\x00".to_vec();
        let idx = 0u32;
        let chain_code = ChainCode::from_bytes(cc_bytes).unwrap();
        let sk = SecpPrivateKey::from_bytes(sk_bytes)
            .expect("We should have implemented that loop in the BIP32 specs");

        Self { depth, parent_fingerprint, idx, chain_code, sk }
    }

    /// #Panics
    /// If the resulting private key is 0, we should have looped to fix that, but this implementation
    /// just panics then
    pub(crate) fn cook_new<F: Fn(&mut HmacSha512) -> ()>(&self, idx: u32, recipe: F) -> Self {
        let parent = self;
        let salt = &parent.chain_code.to_bytes();
        // This unwrap would only panic if the digest algorithm had some inconsistent
        // generic parameters, but the SHA512 we use is consistent with itself
        let mut hasher = HmacSha512::new_varkey(salt).unwrap();

        recipe(&mut hasher);

        let hash_arr = hasher.result().code();
        let hash_bytes = hash_arr.as_slice();

        let sk_bytes = &hash_bytes[..PRIVATE_KEY_SIZE];
        let cc_bytes = &hash_bytes[PRIVATE_KEY_SIZE..];

        let depth = parent.depth + 1;
        let parent_pk = parent.neuter().as_public_key();
        // this uses the compressed pk opposed to the addr that uses the uncompressed pk format:
        let hash = hash160(parent_pk.to_bytes());
        let parent_fingerprint = Vec::from(&hash[..4]);
        let chain_code = ChainCode::from_bytes(cc_bytes).unwrap();
        let sk = (&parent.sk + sk_bytes)
            .expect("We should have implemented that loop in the BIP32 specs");

        Self { depth, parent_fingerprint, idx, chain_code, sk }
    }

    /// Serializes the extended private key according to the format defined in [`BIP32`]
    ///
    /// [`BIP32`]: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#serialization-format
    pub fn to_xprv(&self, network: &dyn Network) -> String {
        let mut res = Vec::with_capacity(XPRV_DATA_SIZE);
        res.extend_from_slice(network.bip32_xprv());
        res.push(self.depth);
        res.extend_from_slice(&self.parent_fingerprint);
        res.extend_from_slice(&self.idx.to_be_bytes());
        res.extend_from_slice(&self.chain_code.to_bytes());
        res.push(SK_PREFIX); // private key is padded to 33 bytes, like a compressed public key
        res.extend_from_slice(&self.sk.to_bytes());

        to_base58check(res)
    }

    /// Deserializes the extended private key from the format defined in [`BIP32`]
    ///
    /// [`BIP32`]: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#serialization-format
    pub fn from_xprv(xprv: &str, network: &dyn Network) -> Fallible<Self> {
        let data = from_base58check(xprv)?;
        ensure!(data.len() == XPRV_DATA_SIZE, "Length of data must be {}", XPRV_DATA_SIZE);

        let expected_prefix = network.bip32_xprv();
        debug_assert_eq!(expected_prefix.len(), VERSION_SIZE);
        debug_assert_eq!(VERSION_SIZE, 4);

        let actual_prefix = &data[0..4];
        ensure!(
            actual_prefix == expected_prefix,
            "Invalid network prefix found: {}",
            hex::encode(actual_prefix)
        );
        let depth = data[4];
        let parent_fingerprint = data[5..9].to_vec();
        let idx = {
            let mut idx_bytes = [0u8; 4];
            idx_bytes.copy_from_slice(&data[9..13]);
            u32::from_be_bytes(idx_bytes)
        };
        let chain_code = {
            let chain_code_bytes = &data[13..45];
            ChainCode::from_bytes(chain_code_bytes)?
        };
        ensure!(data[45] == SK_PREFIX, "xprv must have a private key prefixed with {}", SK_PREFIX);
        let sk = {
            let sk_bytes = &data[46..78];
            SecpPrivateKey::from_bytes(sk_bytes)?
        };

        Ok(Self { depth, parent_fingerprint, idx, chain_code, sk })
    }
}

impl ExtendedPrivateKey<Secp256k1> for SecpExtPrivateKey {
    fn derive_normal_child(&self, idx: i32) -> Fallible<SecpExtPrivateKey> {
        ensure!(idx >= 0, "Derivation index cannot be negative");
        let idx = idx as u32;

        let xprv = self.cook_new(idx, |hasher| {
            hasher.input(&self.sk.public_key().to_bytes());
            hasher.input(&idx.to_be_bytes());
        });

        Ok(xprv)
    }
    fn derive_hardened_child(&self, idx: i32) -> Fallible<SecpExtPrivateKey> {
        ensure!(idx >= 0, "Derivation index cannot be negative");
        let idx = 0x8000_0000u32 + idx as u32;

        let xprv = self.cook_new(idx, |hasher| {
            hasher.input(&[SK_PREFIX]);
            hasher.input(&self.sk.to_bytes());
            hasher.input(&idx.to_be_bytes());
        });

        Ok(xprv)
    }
    fn neuter(&self) -> SecpExtPublicKey {
        let depth = self.depth;
        let parent_fingerprint = self.parent_fingerprint.clone();
        let idx = self.idx;
        let chain_code = self.chain_code.clone();
        let pk = self.sk.public_key();
        SecpExtPublicKey { depth, parent_fingerprint, idx, chain_code, pk }
    }
    fn as_private_key(&self) -> SecpPrivateKey {
        self.sk.clone()
    }
}
