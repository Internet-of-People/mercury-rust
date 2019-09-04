use super::*;

pub const XPUB_DATA_SIZE: usize = 78;

/// Implementation of Secp256k1::ExtendedPublicKey
pub struct SecpExtPublicKey {
    pub(super) depth: u8,
    pub(super) parent_fingerprint: Vec<u8>,
    pub(super) idx: u32,
    pub(super) chain_code: ChainCode,
    pub(super) pk: SecpPublicKey,
}

impl SecpExtPublicKey {
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
        let parent_pk = parent.as_public_key();
        // this uses the compressed pk opposed to the addr that uses the uncompressed pk format:
        let hash = hash160(parent_pk.to_bytes());
        let parent_fingerprint = Vec::from(&hash[..4]);
        let chain_code = ChainCode::from_bytes(cc_bytes).unwrap();
        let pk = (&parent.pk + sk_bytes)
            .expect("We should have implemented that loop in the BIP32 specs");

        Self { depth, parent_fingerprint, idx, chain_code, pk }
    }

    /// Serializes the extended public key according to the format defined in [`BIP32`]
    ///
    /// [`BIP32`]: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#serialization-format
    pub fn to_xpub(&self, network: &dyn Network) -> String {
        let mut res = Vec::with_capacity(XPUB_DATA_SIZE);
        res.extend_from_slice(network.bip32_xpub());
        res.push(self.depth);
        res.extend_from_slice(&self.parent_fingerprint);
        res.extend_from_slice(&self.idx.to_be_bytes());
        res.extend_from_slice(&self.chain_code.to_bytes());
        res.extend_from_slice(&self.pk.to_bytes());

        to_base58check(res)
    }

    /// Deserializes the extended public key from the format defined in [`BIP32`]
    ///
    /// [`BIP32`]: https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki#serialization-format
    pub fn from_xpub(xprv: &str, network: &dyn Network) -> Fallible<Self> {
        let data = from_base58check(xprv)?;
        ensure!(data.len() == XPUB_DATA_SIZE, "Length of data must be {}", XPUB_DATA_SIZE);

        let actual_prefix = &data[0..4];
        ensure!(
            actual_prefix == network.bip32_xpub(),
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
        let pk = {
            let pk_bytes = &data[45..78];
            SecpPublicKey::from_bytes(pk_bytes)?
        };

        Ok(Self { depth, parent_fingerprint, idx, chain_code, pk })
    }
}

impl ExtendedPublicKey<Secp256k1> for SecpExtPublicKey {
    fn derive_normal_child(&self, idx: i32) -> Fallible<SecpExtPublicKey> {
        ensure!(idx >= 0, "Derivation index cannot be negative");
        let idx = idx as u32;

        let xpub = self.cook_new(idx, |hasher| {
            hasher.input(&self.pk.to_bytes());
            hasher.input(&idx.to_be_bytes());
        });

        Ok(xpub)
    }
    fn as_public_key(&self) -> SecpPublicKey {
        self.pk.clone()
    }
}
