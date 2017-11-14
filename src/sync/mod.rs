use common::*;
use error::*;

pub mod imp;



pub trait HashSpace<ObjectType, ReadableHashType>
{
    fn store(&mut self, object: ObjectType) -> Result<ReadableHashType, HashSpaceError>;
    fn resolve(&self, hash: &ReadableHashType) -> Result<ObjectType, HashSpaceError>;
//    fn validate(&self, object: &ObjectType, hash: &str) -> Result<bool, HashSpaceError>;
}


pub trait KeyValueStore<KeyType, ValueType>
{
    // TODO maybe it would be enough to use references instead of consuming params
    fn store(&mut self, key: &KeyType, object: ValueType) -> Result<(), StorageError>;
    fn lookup(&self, key: &KeyType) -> Result<ValueType, StorageError>;
}


pub struct ModularHashSpace<ObjectType, SerializedType, BinaryHashType, ReadableHashType>
{
    serializer: Box< Serializer<ObjectType, SerializedType> >,
    hasher:     Box< Hasher<SerializedType, BinaryHashType> >,
    storage:    Box< KeyValueStore<BinaryHashType, SerializedType> >,
    hash_coder: Box< HashCoder<BinaryHashType, ReadableHashType> >,
}


impl <ObjectType, SerializedType, BinaryHashType, ReadableHashType>
HashSpace<ObjectType, ReadableHashType>
for ModularHashSpace<ObjectType, SerializedType, BinaryHashType, ReadableHashType>
{
    fn store(&mut self, object: ObjectType) -> Result<ReadableHashType, HashSpaceError>
    {
        let serialized_obj = self.serializer.serialize(object)
            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let hash_bytes = self.hasher.get_hash(&serialized_obj)
            .map_err( |e| HashSpaceError::HashError(e) )?;
        self.storage.store( &hash_bytes, serialized_obj )
            .map_err( |e| HashSpaceError::StorageError(e) )?;
        let hash_str = self.hash_coder.encode(&hash_bytes)
            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
        Ok(hash_str)
    }

    fn resolve(&self, hash_str: &ReadableHashType) -> Result<ObjectType, HashSpaceError>
    {
        let hash_bytes = self.hash_coder.decode(&hash_str)
            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
        let serialized_obj = self.storage.lookup(&hash_bytes)
            .map_err( |e| HashSpaceError::StorageError(e) )?;
        let valid_hash = self.hasher.validate(&serialized_obj, &hash_bytes)
            .map_err( |e| HashSpaceError::HashError(e) )?;
        if ! valid_hash
            // TODO consider using a different error code
            { return Err( HashSpaceError::StorageError(StorageError::InvalidKey) ) };

        let object = self.serializer.deserialize(serialized_obj)
            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        Ok(object)
    }

//    fn validate(&self, object: &ObjectType, hash_str: &str) -> Result<bool, HashSpaceError>
//    {
//        let hash_bytes = self.str_coder.decode(&hash_str)
//            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
//        let serialized_obj = self.serializer.serialize(object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )?;
//        let valid = self.hasher.validate(&serialized_obj, &hash_bytes)
//            .map_err( |e| HashSpaceError::HashError(e) )?;
//        Ok(valid)
//    }
}
