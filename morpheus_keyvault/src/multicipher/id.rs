use failure::{ensure, err_msg, Fallible};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::cmp::Ordering;

use super::*;

erased_type! {
    /// Type-erased [`KeyId`]
    ///
    /// [`KeyId`]: ../trait.AsymmetricCrypto.html#associatedtype.KeyId
    pub struct MKeyId {}
}

macro_rules! to_bytes_tuple {
    ($suite:ident, $self_:expr) => {
        (stringify!($suite), reify!($suite, id, $self_).to_bytes())
    };
}

impl Serialize for MKeyId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (discriminator, bytes) = visit!(to_bytes_tuple(self));
        let mut out = bytes.to_vec();
        out.insert(0, discriminator.as_bytes()[0]);
        serializer.serialize_bytes(out.as_slice())
    }
}

fn deser(bytes: Vec<u8>) -> Fallible<MKeyId> {
    ensure!(bytes.is_empty(), "No crypto suite discriminator found");
    let discriminator = bytes[0];
    let data = &bytes[1..];
    let value = match discriminator as char {
        'e' => erase!(e, MKeyId, ed25519::KeyId::from_bytes(data)?),
        'f' => erase!(f, MKeyId, ed25519::KeyId::from_bytes(data)?),
        _ => Err(err_msg(format!(
            "Unknown crypto suite discriminator {}",
            discriminator
        )))?,
    };
    Ok(value)
}

impl<'de> Deserialize<'de> for MKeyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_bytes::deserialize(deserializer)
            .and_then(|b| deser(b).map_err(|e| serde::de::Error::custom(e.to_string())))
    }
}

macro_rules! clone {
    ($suite:ident, $self_:expr) => {{
        let result = reify!($suite, id, $self_).clone();
        erase!($suite, MKeyId, result)
    }};
}

impl Clone for MKeyId {
    fn clone(&self) -> Self {
        visit!(clone(self))
    }
}

macro_rules! eq {
    ($suite:ident, $self_:tt, $other:ident) => {
        reify!($suite, id, $self_).eq(reify!($suite, id, $other))
    };
}

impl PartialEq<MKeyId> for MKeyId {
    fn eq(&self, other: &Self) -> bool {
        if self.suite != other.suite {
            return false;
        }
        visit!(eq(self, other))
    }
}

impl Eq for MKeyId {}

macro_rules! partial_cmp {
    ($suite:ident, $self_:tt, $other:expr) => {
        reify!($suite, id, $self_).partial_cmp(reify!($suite, id, $other))
    };
}

impl PartialOrd<MKeyId> for MKeyId {
    fn partial_cmp(&self, other: &MKeyId) -> Option<Ordering> {
        let suite_order = self.suite.partial_cmp(&other.suite);
        match suite_order {
            Some(Ordering::Equal) => visit!(partial_cmp(self, other)),
            _ => suite_order,
        }
    }
}

macro_rules! hash {
    ($suite:ident, $self_:tt, $state:expr) => {
        reify!($suite, id, $self_).hash($state)
    };
}

impl Hash for MKeyId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.suite.hash(state);
        visit!(hash(self, state));
    }
}

impl From<&MKeyId> for String {
    fn from(src: &MKeyId) -> Self {
        let (discriminator, bytes) = visit!(to_bytes_tuple(src));
        let mut output = multibase::encode(multibase::Base58btc, &bytes);
        output.insert_str(0, discriminator);
        output.insert(0, 'I');
        output
    }
}

impl std::fmt::Display for MKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", String::from(self))
    }
}

impl std::fmt::Debug for MKeyId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        (self as &std::fmt::Display).fmt(f)
    }
}

impl std::str::FromStr for MKeyId {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        let mut chars = src.chars();
        ensure!(chars.next() == Some('I'), "Identifiers must start with 'I'");
        if let Some(discriminator) = chars.next() {
            let (_base, binary) = multibase::decode(chars.as_str())?;
            Ok(match discriminator {
                'e' => erase!(e, MKeyId, ed25519::KeyId::from_bytes(&binary)?),
                'f' => erase!(f, MKeyId, ed25519::KeyId::from_bytes(&binary)?),
                _ => Err(err_msg(format!(
                    "Unknown crypto suite discriminator {}",
                    discriminator
                )))?,
            })
        } else {
            Err(err_msg("No crypto suite discriminator found"))
        }
    }
}

impl From<ed25519::KeyId> for MKeyId {
    fn from(src: ed25519::KeyId) -> Self {
        erase!(e, MKeyId, src)
    }
}
