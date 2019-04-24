use failure::{ensure, Fallible};

use crate::common::*;

pub mod imp;

pub trait HashSpace<ObjectType, ReadableHashType> {
    fn store(&mut self, object: ObjectType) -> Fallible<ReadableHashType>;
    fn resolve(&self, hash: &ReadableHashType) -> Fallible<ObjectType>;
    fn validate(&self, object: &ObjectType, hash: &ReadableHashType) -> Fallible<bool>;
}

pub trait KeyValueStore<KeyType, ValueType> {
    // TODO maybe it would be enough to use references instead of consuming params
    fn store(&mut self, key: &KeyType, object: ValueType) -> Fallible<()>;
    fn lookup(&self, key: &KeyType) -> Fallible<ValueType>;
}

pub struct ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType> {
    //    serializer: Box< Serializer<ObjectType, SerializedType> >,
    hasher: Box<Hasher<SerializedType, BinaryHashType>>,
    storage: Box<KeyValueStore<BinaryHashType, SerializedType>>,
    hash_coder: Box<HashCoder<BinaryHashType, ReadableHashType>>,
}

impl<SerializedType, BinaryHashType, ReadableHashType> HashSpace<SerializedType, ReadableHashType>
    for ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
    fn store(&mut self, serialized_obj: SerializedType) -> Fallible<ReadableHashType> {
        //        let serialized_obj = self.serializer.serialize(object)
        //            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let hash_bytes = self.hasher.get_hash(&serialized_obj)?;
        self.storage.store(&hash_bytes, serialized_obj)?;
        let hash_str = self.hash_coder.encode(&hash_bytes)?;
        Ok(hash_str)
    }

    fn resolve(&self, hash_str: &ReadableHashType) -> Fallible<SerializedType> {
        let hash_bytes = self.hash_coder.decode(&hash_str)?;
        let serialized_obj = self.storage.lookup(&hash_bytes)?;
        let valid_hash = self.hasher.validate(&serialized_obj, &hash_bytes)?;
        ensure!(valid_hash, "Hash is invalid");

        //        let object = self.serializer.deserialize(serialized_obj)
        //            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        Ok(serialized_obj)
    }

    fn validate(
        &self,
        serialized_obj: &SerializedType,
        hash_str: &ReadableHashType,
    ) -> Fallible<bool> {
        let hash_bytes = self.hash_coder.decode(&hash_str)?;
        //        let serialized_obj = self.serializer.serialize(object)
        //            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let valid = self.hasher.validate(&serialized_obj, &hash_bytes)?;
        Ok(valid)
    }
}
