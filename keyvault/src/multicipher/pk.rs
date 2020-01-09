use failure::{ensure, err_msg, Fallible};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::*;

erased_type! {
    /// Type-erased [`PublicKey`]
    ///
    /// [`PublicKey`]: ../trait.AsymmetricCrypto.html#associatedtype.PublicKey
    pub struct MPublicKey {}
}

macro_rules! key_id {
    ($suite:ident, $self_:tt) => {{
        let result = reify!($suite, pk, $self_).key_id();
        erase!($suite, MKeyId, result)
    }};
}

macro_rules! verify {
    ($suite:ident, $self_:tt, $data:ident, $sig:ident) => {
        reify!($suite, pk, $self_).verify($data, reify!($suite, sig, $sig))
    };
}

// TODO this should not be based on the String conversions
impl MPublicKey {
    pub const PREFIX: char = 'p';

    pub fn to_bytes(&self) -> Vec<u8> {
        String::from(self).as_bytes().to_vec()
    }

    pub fn from_bytes(bytes: &[u8]) -> Fallible<Self> {
        let string = String::from_utf8(bytes.to_owned())?;
        string.parse()
    }

    pub fn validate_id(&self, key_id: &MKeyId) -> bool {
        self.key_id() == *key_id
    }
}

impl PublicKey<MultiCipher> for MPublicKey {
    fn key_id(&self) -> MKeyId {
        visit!(key_id(self))
    }
    fn verify<D: AsRef<[u8]>>(&self, data: D, sig: &MSignature) -> bool {
        if self.suite != sig.suite {
            return false;
        }
        visit!(verify(self, data, sig))
    }
}

macro_rules! to_bytes_tuple {
    ($suite:ident, $self_:expr) => {
        (stringify!($suite), reify!($suite, pk, $self_).to_bytes())
    };
}

impl Serialize for MPublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (discriminator, bytes) = visit!(to_bytes_tuple(self));

        let erased = ErasedBytes { discriminator: discriminator.as_bytes()[0], value: bytes };
        erased.serialize(serializer)
    }
}

macro_rules! from_bytes {
    ($suite:ident, $data:expr) => {
        erase!($suite, MPublicKey, <$suite!(pk)>::from_bytes($data)?)
    };
}

fn deser(erased: ErasedBytes) -> Fallible<MPublicKey> {
    let discriminator = erased.discriminator as char;
    let data = &erased.value;
    let value = visit_fac!(
        stringify(discriminator.to_string().as_str()) =>
            from_bytes(data)
    );
    Ok(value)
}

impl<'de> Deserialize<'de> for MPublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ErasedBytes::deserialize(deserializer)
            .and_then(|b| deser(b).map_err(|e| serde::de::Error::custom(e.to_string())))
    }
}

macro_rules! clone {
    ($suite:ident, $self_:expr) => {{
        let result = reify!($suite, pk, $self_).clone();
        erase!($suite, MPublicKey, result)
    }};
}

impl Clone for MPublicKey {
    fn clone(&self) -> Self {
        visit!(clone(self))
    }
}

macro_rules! eq {
    ($suite:ident, $self_:tt, $other:ident) => {
        reify!($suite, pk, $self_).eq(reify!($suite, pk, $other))
    };
}

impl PartialEq<MPublicKey> for MPublicKey {
    fn eq(&self, other: &Self) -> bool {
        if self.suite != other.suite {
            return false;
        }
        visit!(eq(self, other))
    }
}

impl Eq for MPublicKey {}

impl From<&MPublicKey> for String {
    fn from(src: &MPublicKey) -> Self {
        let (discriminator, bytes) = visit!(to_bytes_tuple(src));
        let mut output = multibase::encode(multibase::Base58btc, &bytes);
        output.insert_str(0, discriminator);
        output.insert(0, MPublicKey::PREFIX);
        output
    }
}

impl From<MPublicKey> for String {
    fn from(src: MPublicKey) -> Self {
        (&src).into()
    }
}

impl std::fmt::Display for MPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl std::fmt::Debug for MPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (self as &dyn std::fmt::Display).fmt(f)
    }
}

impl std::str::FromStr for MPublicKey {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut chars = src.chars();
        ensure!(
            chars.next() == Some(Self::PREFIX),
            "Public keys must start with '{}'",
            Self::PREFIX
        );
        if let Some(discriminator) = chars.next() {
            let (_base, binary) = multibase::decode(chars.as_str())?;
            let ret = visit_fac!(
                stringify(discriminator.to_string().as_str()) =>
                    from_bytes(binary)
            );
            Ok(ret)
        } else {
            Err(err_msg("No crypto suite discriminator found"))
        }
    }
}

impl From<EdPublicKey> for MPublicKey {
    fn from(src: EdPublicKey) -> Self {
        erase!(e, MPublicKey, src)
    }
}

#[cfg(test)]
mod test {
    mod parse_key_id {
        use crate::ed25519::EdPublicKey;
        use crate::multicipher::MPublicKey;

        #[allow(dead_code)]
        fn case(input: &str, pk_hex: &str) {
            let pk_bytes = hex::decode(pk_hex).unwrap();
            let pk1 = EdPublicKey::from_bytes(&pk_bytes).unwrap();
            let erased_pk1 = MPublicKey::from(pk1);
            assert_eq!(erased_pk1.to_string(), input);

            let erased_pk2 = input.parse::<MPublicKey>().unwrap();
            assert_eq!(erased_pk2, erased_pk1);
        }

        #[test]
        fn test_1() {
            case(
                "pez11111111111111111111111111111111",
                "0000000000000000000000000000000000000000000000000000000000000000",
            );
        }

        #[test]
        fn test_2() {
            case(
                "pezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb",
                "8fe9693f8fa62a4305a140b9764c5ee01e455963744fe18204b4fb948249308a",
            );
        }

        #[test]
        fn test_3() {
            case(
                "pezFVen3X669xLzsi6N2V91DoiyzHzg1uAgqiT8jZ9nS96Z",
                "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
            );
        }

        #[test]
        fn test_4() {
            case(
                "pez586Z7H2vpX9qNhN2T4e9Utugie3ogjbxzGaMtM3E6HR5",
                "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
            );
        }

        #[test]
        fn test_5() {
            case(
                "pezHyx62wPQGyvXCoihZq1BrbUjBRh2LuNxWiiqMkfAuSZr",
                "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
            );
        }

        #[test]
        fn discriminator_matters() {
            let pk1 = "pez11111111111111111111111111111111".parse::<MPublicKey>().unwrap();
            let pk2 = "pfz11111111111111111111111111111111".parse::<MPublicKey>().unwrap();
            assert_ne!(pk1, pk2);
        }

        #[test]
        #[should_panic(expected = "Unknown crypto suite discriminator \\'g\\'")]
        fn invalid_discriminator() {
            let _pk =
                "pgzAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb".parse::<MPublicKey>().unwrap();
        }

        #[test]
        #[should_panic(expected = "No crypto suite discriminator found")]
        fn missing_discriminator() {
            let _pk = "p".parse::<MPublicKey>().unwrap();
        }

        #[test]
        #[should_panic(expected = "Public keys must start with \\'p\\'")]
        fn invalid_type() {
            let _pk = "Fez21JXEtMzXjbCK6BAYFU9ewX".parse::<MPublicKey>().unwrap();
        }

        #[test]
        #[should_panic(expected = "Public keys must start with \\'p\\'")]
        fn empty() {
            let _pk = "".parse::<MPublicKey>().unwrap();
        }
    }

    mod serde_key_id {
        use crate::multicipher::MPublicKey;

        #[test]
        fn messagepack_serialization() {
            let pk_str = "pezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb";
            let pk = pk_str.parse::<MPublicKey>().unwrap();
            let pk_bin = rmp_serde::to_vec(&pk).unwrap();

            assert_eq!(
                pk_bin,
                vec![
                    146, 101, 196, 32, 143, 233, 105, 63, 143, 166, 42, 67, 5, 161, 64, 185, 118,
                    76, 94, 224, 30, 69, 89, 99, 116, 79, 225, 130, 4, 180, 251, 148, 130, 73, 48,
                    138
                ]
            );

            let pk_deser: MPublicKey = rmp_serde::from_slice(&pk_bin).unwrap();
            let pk_tostr = pk_deser.to_string();
            assert_eq!(pk, pk_deser);
            assert_eq!(pk_str, pk_tostr);
        }

        #[test]
        fn json_serialization() {
            let pk_str = "pezAgmjPHe5Qs4VakvXHGnd6NsYjaxt4suMUtf39TayrSfb";
            let pk = pk_str.parse::<MPublicKey>().unwrap();
            let pk_bin = serde_json::to_vec(&pk).unwrap();

            assert_eq!(pk_bin, br#"{"discriminator":101,"value":[143,233,105,63,143,166,42,67,5,161,64,185,118,76,94,224,30,69,89,99,116,79,225,130,4,180,251,148,130,73,48,138]}"#.to_vec());

            let pk_deser: MPublicKey = serde_json::from_slice(&pk_bin).unwrap();
            let pk_tostr = pk_deser.to_string();
            assert_eq!(pk, pk_deser);
            assert_eq!(pk_str, pk_tostr);
        }
    }
}
