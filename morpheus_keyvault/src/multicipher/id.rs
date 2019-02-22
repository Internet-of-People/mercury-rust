use serde::{Serialize, Serializer};
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
