use std::collections::HashMap;
use std::hash::Hash;

use common::*;
use error::*;



pub trait HashSpace<ObjectType>
{
    fn store(&mut self, object: ObjectType) -> Result<String, HashSpaceError>;
    fn resolve(&self, hash: &str) -> Result<ObjectType, HashSpaceError>;
    fn validate(&self, object: &ObjectType, hash: &str) -> Result<bool, HashSpaceError>;
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
        let serialized_obj = self.serializer.serialize(&object)
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

        let object = self.serializer.deserialize(&serialized_obj)
            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        Ok(object)
    }

    fn validate(&self, object: &ObjectType, hash_str: &str) -> Result<bool, HashSpaceError>
    {
        let hash_bytes = self.str_coder.decode(&hash_str)
            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
        let serialized_obj = self.serializer.serialize(&object)
            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let valid = self.hasher.validate(&serialized_obj, &hash_bytes)
            .map_err( |e| HashSpaceError::HashError(e) )?;
        Ok(valid)
    }
}



pub struct InMemoryStore<KeyType, ValueType>
{
    map: HashMap<KeyType, ValueType>,
}

impl<KeyType, ValueType> InMemoryStore<KeyType, ValueType>
    where KeyType: Eq + Hash
{
    pub fn new() -> Self
        { InMemoryStore{ map: HashMap::new() } }
}

impl<KeyType, ValueType>
KeyValueStore<KeyType, ValueType>
for InMemoryStore<KeyType, ValueType>
    where KeyType: Eq + Hash + Clone,
          ValueType: Clone
{
    fn store(&mut self, key: &KeyType, object: ValueType) -> Result<(), StorageError>
    {
        self.map.insert(key.to_owned(), object );
        Ok( () )
    }

    fn lookup(&self, key: &KeyType) -> Result<ValueType, StorageError>
    {
        self.map.get(&key)
            .map( |v| v.to_owned() )
            .ok_or(StorageError::InvalidKey)
    }
}



#[cfg(test)]
mod tests
{
    use super::*;
    use super::super::*;


    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person
    {
        name:  String,
        phone: String,
        age:   u16,
    }


    #[test]
    fn test_storage()
    {
        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let hash = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut storage: InMemoryStore<Vec<u8>,Person> = InMemoryStore::new();
        let store_res = storage.store( &hash, object.clone() );
        assert!( store_res.is_ok() );
        let lookup_res = storage.lookup(&hash);
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
    }

    #[test]
    fn test_hashspace()
    {
        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: CompositeHashSpace<Person, Vec<u8>, Vec<u8>> = CompositeHashSpace{
            serializer: Box::new( SerdeJsonSerializer{} ),
            hasher:     Box::new( MultiHasher::new(multihash::Hash::Keccak512) ),
            storage:    Box::new(store),
            str_coder:  Box::new( MultiBaseStringCoder::new(multibase::Base64) ) };

        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let store_res = hashspace.store( object.clone() );
        assert!( store_res.is_ok() );
        let hash = store_res.unwrap();
        let lookup_res = hashspace.resolve(&hash);
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
        let validate_res = hashspace.validate(&object, &hash);
        assert!( validate_res.is_ok() );
        assert!( validate_res.unwrap() );
    }
}
