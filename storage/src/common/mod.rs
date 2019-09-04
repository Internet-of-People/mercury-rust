use failure::Fallible;

use crate::meta;
use crate::meta::{Attribute, AttributeValue};

pub mod imp;

pub trait Data {
    fn blob(&self) -> &[u8];
    fn attributes<'a>(&'a self) -> Box<dyn 'a + Iterator<Item = &dyn Attribute>>;

    // Convenience function to access attributes by name/path
    fn first_attrval_by_name(&self, name: &str) -> Option<AttributeValue> {
        meta::iter_first_attrval_by_name(self.attributes(), name)
    }

    fn first_attrval_by_path(&self, path: &[&str]) -> Option<AttributeValue> {
        meta::iter_first_attrval_by_path(self.attributes(), path)
    }
}

// De/Serialize in-memory data from/to a memory-independent storable
// (binary, e.g. bson or json-utf8) representation
pub trait Serializer<ObjectType, SerializedType> {
    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: ObjectType) -> Fallible<SerializedType>;
    fn deserialize(&self, serialized_object: SerializedType) -> Fallible<ObjectType>;
}

// Provide (binary, e.g. SHA2) hash for (binary) data and validate hash against data
pub trait Hasher<ObjectType, HashType> {
    fn get_hash(&self, object: &ObjectType) -> Fallible<HashType>;
    fn validate(&self, object: &ObjectType, hash: &HashType) -> Fallible<bool>;
}

// Provide human-readable (e.g. Base64) representation of (binary) hashes
pub trait HashCoder<BinaryHashType, ReadableHashType> {
    fn encode(&self, hash_bytes: &BinaryHashType) -> Fallible<ReadableHashType>;
    fn decode(&self, hash_str: &ReadableHashType) -> Fallible<BinaryHashType>;
}
