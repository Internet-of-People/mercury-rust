use super::*;

/// The size of the key identifier in bytes. Since a version byte is prepended to the
/// hash result, it is not a standard size.
pub const KEY_ID_SIZE: usize = 20 + VERSION_SIZE;

/// The serialized byte representation for the current version of the hash algorithm
/// applied on the public key to obtain the key identifier
pub const KEY_ID_VERSION1: u8 = b'\x01';

/// Implementation of Secp256k1::KeyId
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SecpKeyId(Vec<u8>);

impl SecpKeyId {
    /// The key id serialized in a format that can be fed to [`from_bytes`]
    ///
    /// [`from_bytes`]: #method.from_bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.clone()
    }

    /// Creates a key id from a byte slice possibly returned by the [`to_bytes`] method.
    ///
    /// # Error
    /// If `bytes` is not [`KEY_ID_SIZE`] long
    ///
    /// [`to_bytes`]: #method.to_bytes
    /// [`KEY_ID_SIZE`]: ../constant.KEY_ID_SIZE
    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D) -> Fallible<Self> {
        let bytes = bytes.as_ref();
        ensure!(bytes.len() == KEY_ID_SIZE, "Identifier length is not {}", KEY_ID_SIZE);
        ensure!(
            bytes[0] == KEY_ID_VERSION1,
            "Only identifier version {:x} is supported",
            KEY_ID_VERSION1
        );
        Ok(Self(bytes.to_owned()))
    }

    /// Serializes the key identifier as a `p2pkh` bitcoin address
    ///
    /// # Panics
    /// If internal invariants of the key id format are not maintained because of a bug
    pub fn to_p2pkh_addr(&self, network: &dyn Network) -> String {
        assert_eq!(self.0[0], KEY_ID_VERSION1);
        assert_eq!(self.0.len(), KEY_ID_SIZE);

        let prefix = network.p2pkh_addr();
        debug_assert_eq!(prefix.len(), ADDR_PREFIX_SIZE);
        let mut address = Vec::with_capacity(ADDR_PREFIX_SIZE + KEY_ID_SIZE - VERSION_SIZE);
        address.extend_from_slice(prefix);
        address.extend_from_slice(&self.0[VERSION_SIZE..]);

        to_base58check(address)
    }

    /// Deserializes the key identifier from a `p2pkh` bitcoin address
    pub fn from_p2pkh_addr(addr: &str, network: &dyn Network) -> Fallible<Self> {
        let expected_prefix = network.p2pkh_addr();
        debug_assert_eq!(expected_prefix.len(), ADDR_PREFIX_SIZE);
        debug_assert_eq!(ADDR_PREFIX_SIZE, 1);

        let data = from_base58check(addr)?;
        ensure!(
            data.len() == ADDR_PREFIX_SIZE + KEY_ID_SIZE - VERSION_SIZE,
            "Invalid length of address"
        );

        let actual_prefix = &data[0..1];
        ensure!(
            actual_prefix == expected_prefix,
            "Invalid network prefix found: {}",
            hex::encode(actual_prefix)
        );

        let mut id = Vec::with_capacity(KEY_ID_SIZE);
        id.push(KEY_ID_VERSION1);
        id.extend_from_slice(&data[1..]);

        Ok(Self(id))
    }
}

impl From<&SecpPublicKey> for SecpKeyId {
    // https://en.bitcoin.it/wiki/Technical_background_of_version_1_Bitcoin_addresses
    fn from(pk: &SecpPublicKey) -> SecpKeyId {
        let hash = hash160(&pk.to_bytes()[..]);

        let mut id = Vec::with_capacity(KEY_ID_SIZE);
        id.push(KEY_ID_VERSION1);
        id.extend_from_slice(&*hash);

        SecpKeyId(id)
    }
}
