use common::*;
use error::*;

pub mod imp;



pub trait HashSpace<ObjectType>
{
    fn store(&mut self, object: ObjectType) -> Result<String, HashSpaceError>;
    fn resolve(&self, hash: &str) -> Result<ObjectType, HashSpaceError>;
//    fn validate(&self, object: &ObjectType, hash: &str) -> Result<bool, HashSpaceError>;
}


pub trait KeyValueStore<KeyType, ValueType>
{
    // TODO maybe it would be enough to use references instead of consuming params
    fn store(&mut self, key: &KeyType, object: ValueType) -> Result<(), StorageError>;
    fn lookup(&self, key: &KeyType) -> Result<ValueType, StorageError>;
}


pub struct CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    serializer: Box< Serializer<ObjectType, SerializedType> >,
    hasher:     Box< Hasher<SerializedType, HashType> >,
    storage:    Box< KeyValueStore<HashType, SerializedType> >,
    str_coder:  Box< StringCoder<HashType> >,
}


impl <ObjectType, SerializedType, HashType>
HashSpace<ObjectType>
for CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    fn store(&mut self, object: ObjectType) -> Result<String, HashSpaceError>
    {
        let serialized_obj = self.serializer.serialize(object)
            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let obj_hash = self.hasher.get_hash(&serialized_obj)
            .map_err( |e| HashSpaceError::HashError(e) )?;
        self.storage.store( &obj_hash, serialized_obj )
            .map_err( |e| HashSpaceError::StorageError(e) )?;
        let hash_str = self.str_coder.encode(&obj_hash)
            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
        Ok(hash_str)
    }

    fn resolve(&self, hash_str: &str) -> Result<ObjectType, HashSpaceError>
    {
        let hash_bytes = self.str_coder.decode(&hash_str)
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
