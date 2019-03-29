//! Generic data structures and algorithms for [BIP-0032](
//! https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki) and
//! [SLIP-0010](https://github.com/satoshilabs/slips/blob/master/slip-0010.md) compatible
//! child-key derivation for building hierarchical deterministic wallets.

use failure::Fallible;
use std::str::FromStr;

use crate::{ExtendedPrivateKey, ExtendedPublicKey, KeyDerivationCrypto, PublicKey, Seed};

/// An item in the [BIP-0032](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki)
/// derivation [path](struct.Path.html). A combination of a 31-bit unsigned integer and a flag, which derivation
/// method (normal or hardened) to use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChildIndex {
    /// Normal (aka. public) derivation allows deriving a child extended public key
    /// based on a parent extended public key.
    Normal(i32),
    /// Hardened (aka. private) derivation only allows deriving a child extended private key
    /// based on a parent extended private key, but having only an extended public key does
    /// not help deriving hardened children of any kind.
    Hardened(i32),
}

/// An absolute [BIP32](https://github.com/bitcoin/bips/blob/master/bip-0032.mediawiki) derivation
/// path that starts from the master keypair. This is useful to create a [hierarchical deterministic
/// tree](https://bitcoin.org/en/developer-guide#hierarchical-deterministic-key-creation) of keypairs
/// for [any cryptography](https://github.com/satoshilabs/slips/blob/master/slip-0010.md) that supports
/// child key derivation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Path {
    path: Vec<ChildIndex>,
}

/// Generic implementations of applying a [BIP32 path](struct.Path.html) to a seed using a cryptography that
/// supports child key derivation.
pub trait Bip32Path<C: KeyDerivationCrypto + ?Sized> {
    /// Calculate an extended private key from a [`Seed`] and a [`Path`]. The result
    /// can be used as an intermediate step for further child key derivations using the low-level
    /// API defined in the [`ExtendedPrivateKey`] trait.
    ///
    /// [`Seed`]: ../struct.Seed.html
    /// [`Path`]: struct.Path.html
    /// [`ExtendedPrivateKey`]: ../trait.ExtendedPrivateKey.html
    fn calc_ext_priv_key(seed: &Seed, path: &Path) -> Fallible<C::ExtendedPrivateKey>;
    /// Calculate an extended public key from a [`Seed`] and a [`Path`]. The result
    /// can be used as an intermediate step for normal derivations of further extended public
    /// keys using the low-level API defined in the [`ExtendedPublicKey`] trait.
    ///
    /// [`Seed`]: ../struct.Seed.html
    /// [`Path`]: struct.Path.html
    /// [`ExtendedPublicKey`]: ../trait.ExtendedPublicKey.html
    fn calc_ext_pub_key(seed: &Seed, path: &Path) -> Fallible<C::ExtendedPublicKey>;
    /// Calculate a private key from a [`Seed`] and a [`Path`]. The result can be used
    /// to authentication and decryption of data using the low-level API defined in the
    /// [`PrivateKey`] trait.
    ///
    /// [`Seed`]: ../struct.Seed.html
    /// [`Path`]: struct.Path.html
    /// [`PrivateKey`]: ../trait.PrivateKey.html
    fn calc_priv_key(seed: &Seed, path: &Path) -> Fallible<C::PrivateKey>;
    /// Calculate a public key from a [`Seed`] and a [`Path`]. The result can be used to
    /// verify authentication and encryption of data using the low-level API defined in the
    /// [`PublicKey`] trait.
    ///
    /// [`Seed`]: ../struct.Seed.html
    /// [`Path`]: struct.Path.html
    /// [`PublicKey`]: ../trait.PublicKey.html
    fn calc_pub_key(seed: &Seed, path: &Path) -> Fallible<C::PublicKey>;
    /// Calculate a key id (aka. address, fingerprint) from a [`Seed`] and a [`Path`].
    /// The result can be used to check if a revealed [`PublicKey`] matches this digest.
    ///
    /// [`Seed`]: ../struct.Seed.html
    /// [`Path`]: struct.Path.html
    /// [`PublicKey`]: ../trait.PublicKey.html
    fn calc_key_id(seed: &Seed, path: &Path) -> Fallible<C::KeyId>;
}

fn is_hardened_suffix_char(c: char) -> bool {
    ['\'', 'h', 'H'].contains(&c)
}

impl FromStr for ChildIndex {
    type Err = failure::Error;
    fn from_str(mut src: &str) -> Result<Self, Self::Err> {
        let hardened = src.ends_with(is_hardened_suffix_char);
        if hardened {
            src = &src[..src.len() - 1];
        };
        let idx = src.parse::<i32>()?;
        if idx < 0 {
            bail!("BIP32 derivation index cannot be negative");
        }
        Ok(if hardened { ChildIndex::Hardened(idx) } else { ChildIndex::Normal(idx) })
    }
}

impl FromStr for Path {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut pieces = src.split('/');

        let first_opt = pieces.next();
        if let Some(first) = first_opt {
            if first != "m" {
                bail!("BIP32 derivation path needs to start with 'm'");
            }
        } else {
            bail!("BIP32 derivation path cannot be empty");
        }

        let (mut successes, errors): (Vec<_>, Vec<_>) =
            pieces.map(|p: &str| (p, p.parse::<ChildIndex>())).partition(|(_p, i)| i.is_ok());

        if !errors.is_empty() {
            bail!("BIP32 derivation path contains invalid child indices: {:?}", errors);
        }

        // because of the above partitioning, successes only contain parse results
        // that can be unwrapped without causing a panic
        let path = successes.drain(..).map(|(_p, i)| i.unwrap()).collect();
        Ok(Path { path })
    }
}

impl<C: KeyDerivationCrypto + ?Sized> Bip32Path<C> for C {
    fn calc_ext_priv_key(seed: &Seed, path: &Path) -> Fallible<C::ExtendedPrivateKey> {
        let mut xprv = C::master(seed);
        for item in &path.path {
            xprv = match *item {
                ChildIndex::Hardened(idx) => xprv.derive_hardened_child(idx),
                ChildIndex::Normal(idx) => xprv.derive_normal_child(idx),
            }?
        }
        Ok(xprv)
    }

    fn calc_ext_pub_key(seed: &Seed, path: &Path) -> Fallible<C::ExtendedPublicKey> {
        let xprv = Self::calc_ext_priv_key(seed, path)?;
        Ok(xprv.neuter())
    }

    fn calc_priv_key(seed: &Seed, path: &Path) -> Fallible<C::PrivateKey> {
        let xprv = Self::calc_ext_priv_key(seed, path)?;
        Ok(xprv.as_private_key())
    }

    fn calc_pub_key(seed: &Seed, path: &Path) -> Fallible<C::PublicKey> {
        let xprv = Self::calc_ext_priv_key(seed, path)?;
        Ok(xprv.neuter().as_public_key())
    }

    fn calc_key_id(seed: &Seed, path: &Path) -> Fallible<C::KeyId> {
        let xprv = Self::calc_ext_priv_key(seed, path)?;
        Ok(xprv.neuter().as_public_key().key_id())
    }
}

#[cfg(test)]
mod tests {
    use super::{ChildIndex, Path};
    use crate::*;
    use std::fmt;

    struct TestCrypto {}

    #[derive(Clone, Hash, Eq, PartialEq)]
    struct TestKeyId(String);

    impl fmt::Debug for TestKeyId {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(&self.0)
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TestSignature {
        data: Vec<u8>,
        pub_key: TestPublicKey,
    }

    #[derive(Clone, Eq, PartialEq)]
    struct TestPrivateKey(String);

    impl fmt::Debug for TestPrivateKey {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(format_args!("sk({:})", self.0))
        }
    }

    impl PrivateKey<TestCrypto> for TestPrivateKey {
        fn public_key(&self) -> TestPublicKey {
            TestPublicKey(self.0.clone())
        }
        fn sign<D: AsRef<[u8]>>(&self, data: D) -> TestSignature {
            TestSignature { data: data.as_ref().to_owned(), pub_key: self.public_key() }
        }
    }

    #[derive(Clone, Eq, PartialEq)]
    struct TestPublicKey(String);

    impl fmt::Debug for TestPublicKey {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(format_args!("pk({:})", self.0))
        }
    }

    impl PublicKey<TestCrypto> for TestPublicKey {
        fn key_id(&self) -> TestKeyId {
            TestKeyId(format!("id({0})", self.0))
        }
        fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &TestSignature) -> bool {
            sig.data.as_slice() == data.as_ref() && *self == sig.pub_key
        }
    }

    impl AsymmetricCrypto for TestCrypto {
        type KeyId = TestKeyId;
        type PrivateKey = TestPrivateKey;
        type PublicKey = TestPublicKey;
        type Signature = TestSignature;
    }

    #[derive(Clone, Eq, PartialEq)]
    struct TestXprv(TestPrivateKey);

    impl fmt::Debug for TestXprv {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(format_args!("xprv({:?})", self.0))
        }
    }

    impl ExtendedPrivateKey<TestCrypto> for TestXprv {
        fn derive_normal_child(&self, idx: i32) -> Fallible<TestXprv> {
            Ok(TestXprv(TestPrivateKey(format!("{}/{}", (self.0).0, idx))))
        }
        fn derive_hardened_child(&self, idx: i32) -> Fallible<TestXprv> {
            Ok(TestXprv(TestPrivateKey(format!("{}/{}'", (self.0).0, idx))))
        }
        fn neuter(&self) -> TestXpub {
            TestXpub(TestPublicKey((self.0).0.clone()))
        }
        fn as_private_key(&self) -> TestPrivateKey {
            self.0.clone()
        }
    }

    #[derive(Clone, Eq, PartialEq)]
    struct TestXpub(TestPublicKey);

    impl fmt::Debug for TestXpub {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_fmt(format_args!("xpub({:?})", self.0))
        }
    }

    impl ExtendedPublicKey<TestCrypto> for TestXpub {
        fn derive_normal_child(&self, idx: i32) -> Fallible<TestXpub> {
            Ok(TestXpub(TestPublicKey(format!("{}/{}", (self.0).0, idx))))
        }
        fn as_public_key(&self) -> TestPublicKey {
            self.0.clone()
        }
    }

    impl KeyDerivationCrypto for TestCrypto {
        type ExtendedPrivateKey = TestXprv;
        type ExtendedPublicKey = TestXpub;

        fn master(_seed: &Seed) -> TestXprv {
            TestXprv(TestPrivateKey("m".to_owned()))
        }
    }

    #[test]
    fn childidx_fromstr() {
        assert_eq!("0".parse::<ChildIndex>().unwrap(), ChildIndex::Normal(0));
        assert_eq!("0h".parse::<ChildIndex>().unwrap(), ChildIndex::Hardened(0));
        assert_eq!("0H".parse::<ChildIndex>().unwrap(), ChildIndex::Hardened(0));
        assert_eq!("0'".parse::<ChildIndex>().unwrap(), ChildIndex::Hardened(0));
        assert_eq!("2147483647".parse::<ChildIndex>().unwrap(), ChildIndex::Normal(2_147_483_647));
        assert_eq!(
            "2147483647'".parse::<ChildIndex>().unwrap(),
            ChildIndex::Hardened(2_147_483_647)
        );
        assert!("2147483648".parse::<ChildIndex>().is_err());
        assert!("-1".parse::<ChildIndex>().is_err());
        assert!("-2147483648".parse::<ChildIndex>().is_err());
        assert!("522147483648".parse::<ChildIndex>().is_err());
        assert!("h".parse::<ChildIndex>().is_err());
        assert!("-h".parse::<ChildIndex>().is_err());
        assert!("0a".parse::<ChildIndex>().is_err());
        assert!("a".parse::<ChildIndex>().is_err());
    }

    #[test]
    fn path_fromstr() {
        assert_eq!("m".parse::<Path>().unwrap(), Path { path: Default::default() });
        assert_eq!("m/0".parse::<Path>().unwrap(), Path { path: vec![ChildIndex::Normal(0)] });
        assert_eq!("m/44'".parse::<Path>().unwrap(), Path { path: vec![ChildIndex::Hardened(44)] });
        assert_eq!(
            "m/44'/0h/0H/0".parse::<Path>().unwrap(),
            Path {
                path: vec![
                    ChildIndex::Hardened(44),
                    ChildIndex::Hardened(0),
                    ChildIndex::Hardened(0),
                    ChildIndex::Normal(0)
                ]
            }
        );
        assert_eq!(
            "m/2147483647'/2147483647".parse::<Path>().unwrap(),
            Path {
                path: vec![ChildIndex::Hardened(2_147_483_647), ChildIndex::Normal(2_147_483_647)]
            }
        );
        assert!("".parse::<Path>().is_err());
        assert!("M".parse::<Path>().is_err());
        assert!("m/".parse::<Path>().is_err());
        assert!("m/m".parse::<Path>().is_err());
        assert!("m/2147483648".parse::<Path>().is_err());
        assert!("m/522147483648".parse::<Path>().is_err());
    }

    macro_rules! assert_fmt {
        ($actual:expr, $($arg:tt)+) => {
            assert_eq!(format!("{:?}", $actual), format!($($arg)+));
        }
    }

    fn test_path(path_str: &str) {
        use super::Bip32Path;
        let seed = crate::Seed::generate_new();
        let path = path_str.parse::<Path>().unwrap();
        assert_fmt!(TestCrypto::calc_ext_priv_key(&seed, &path).unwrap(), "xprv(sk({}))", path_str);
        assert_fmt!(TestCrypto::calc_ext_pub_key(&seed, &path).unwrap(), "xpub(pk({}))", path_str);
        assert_fmt!(TestCrypto::calc_priv_key(&seed, &path).unwrap(), "sk({})", path_str);
        assert_fmt!(TestCrypto::calc_pub_key(&seed, &path).unwrap(), "pk({})", path_str);
        assert_fmt!(TestCrypto::calc_key_id(&seed, &path).unwrap(), "id({})", path_str);
    }

    #[test]
    fn apply_path() {
        test_path("m");
        test_path("m/0'");
        test_path("m/44'/0'/0'/0/0");
    }
}
