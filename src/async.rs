use std::collections::HashMap;
use std::hash::Hash;

use futures::future::Future;

use super::*;
use common::*;
use error::*;


pub trait HashSpace<ObjectType, HashType>
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=HashType, Error=HashSpaceError> >;
    fn lookup(&self, hash: &HashType)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >;
    fn validate(&self, object: &ObjectType, hash: &HashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >;
}



pub trait KeyValueStore<KeyType, ValueType>
{
    fn store(&mut self, key: &KeyType, object: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn lookup(&self, key: &KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >;
}



pub struct CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    serializer: Box< Serializer<ObjectType, SerializedType> >,
    hasher:     Box< Hasher<SerializedType, HashType> >,
    storage:    Box< KeyValueStore<HashType, SerializedType> >,
}


//impl <ObjectType, SerializedType, HashType>
//HashSpace<ObjectType, HashType>
//for CompositeHashSpace<ObjectType, SerializedType, HashType>
//{
//    fn store(&mut self, object: ObjectType)
//        -> Box< Future<Item=HashType, Error=HashSpaceError> >
//    {
//        let result = self.serializer.serialize(&object)
//            .map_err( |e| HashSpaceError::SerializerError(e) );
//// TODO            .and_then( |serialized_obj| self.hasher.hash(&serialized_obj) )
//
//        let obj_hash = self.hasher.hash(&serialized_obj)
//            .map_err( |e| HashSpaceError::HashError(e) );
//        self.storage.store( &obj_hash, serialized_obj )
//            .map_err( |e| HashSpaceError::StorageError(e) )?;
//        Ok(obj_hash)
//    }
//
//    fn lookup(&self, hash: &HashType)
//        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >
//    {
//        let serialized_obj = self.storage.lookup(&hash)
//            .map_err( |e| HashSpaceError::StorageError(e) )?;
//        let object = self.serializer.deserialize(&serialized_obj)
//            .map_err( |e| HashSpaceError::SerializerError(e) )?;
//        Ok(object)
//    }
//
//    fn validate(&self, object: &ObjectType, hash: &HashType)
//        -> Box< Future<Item=bool, Error=HashSpaceError> >
//    {
//        let serialized_obj = self.serializer.serialize(&object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )?;
//        let valid = self.hasher.validate(&serialized_obj, &hash)
//            .map_err( |e| HashSpaceError::HashError(e) )?;
//        Ok(valid)
//    }
//}
//
//
//
//pub struct MultiHasher
//{
//    hash_algorithm: multihash::Hash,
//}
//
//impl MultiHasher
//{
//    fn to_hasher_error(error: multihash::Error) -> HashError
//    {
//        match error {
//            multihash::Error::BadInputLength    => HashError::BadInputLength,
//            multihash::Error::UnkownCode        => HashError::UnknownCode,
//            multihash::Error::UnsupportedType   => HashError::UnsupportedType,
//        }
//    }
//}
//
//impl Hasher<Vec<u8>, Vec<u8>> for MultiHasher
//{
//    fn hash(&self, data: &Vec<u8>)
//        -> Box< Future<Item=Vec<u8>, Error=HashError> >
//    {
//        multihash::encode(self.hash_algorithm, data)
//            .map_err(MultiHasher::to_hasher_error)
//    }
//
//    fn validate(&self, data: &Vec<u8>, expected_hash: &Vec<u8>)
//        -> Box< Future<Item=bool, Error=HashError> >
//    {
//        //        // TODO should we do this here or just drop this step and check hash equality?
//        //        let decode_result = decode(expected_hash)
//        //            .map_err(MultiHasher::to_hasher_error)?;
//        //        if decode_result.alg != self.hash_algorithm
//        //            { return Err(HashError::UnsupportedType); }
//
//        let calculated_hash = multihash::encode(self.hash_algorithm, data)
//            .map_err(MultiHasher::to_hasher_error)?;
//        Ok(*expected_hash == calculated_hash)
//    }
//}
//
//
//
//// TODO this struct should be independent of the serialization format (e.g. JSON):
////      Maybe should contain Box<serde::ser::De/Serializer> data members
//pub struct SerdeJsonSerializer;
//
//impl SerdeJsonSerializer
//{
//    fn to_serializer_error(error: serde_json::Error) -> SerializerError {
//        SerializerError::SerializationError( Box::new(error) )
//    }
//}
//
//impl<ObjectType> Serializer<ObjectType, Vec<u8>> for SerdeJsonSerializer
//    where ObjectType: serde::Serialize + serde::de::DeserializeOwned
//{
//    fn serialize(&self, object: &ObjectType)
//        -> Box< Future<Item=Vec<u8>, Error=SerializerError> >
//    {
//        serde_json::to_string(&object)
//            .map( |str| str.into_bytes() )
//            .map_err(SerdeJsonSerializer::to_serializer_error)
//    }
//
//    fn deserialize(&self, serialized_object: &Vec<u8>)
//        -> Box< Future<Item=ObjectType, Error=SerializerError> >
//    {
//        let json_string = String::from_utf8(serialized_object.clone() )
//            .map_err(|e| SerializerError::DeserializationError( Box::new(e) ) )?;
//        serde_json::from_str(& json_string)
//            .map_err(SerdeJsonSerializer::to_serializer_error)
//    }
//}
//
//
//
//pub struct InMemoryStore<KeyType, ValueType>
//{
//    map: HashMap<KeyType, ValueType>,
//}
//
//impl<KeyType, ValueType> InMemoryStore<KeyType, ValueType>
//    where KeyType: Eq + Hash
//{
//    pub fn new() -> Self
//    { InMemoryStore{ map: HashMap::new() } }
//}
//
//impl<KeyType, ValueType>
//KeyValueStore<KeyType, ValueType>
//for InMemoryStore<KeyType, ValueType>
//    where KeyType: Eq + Hash + Clone,
//          ValueType: Clone
//{
//    fn store(&mut self, key: &KeyType, object: ValueType)
//        -> Box< Future<Item=(), Error=StorageError> >
//    {
//        self.map.insert(key.to_owned(), object );
//        Ok(true)
//    }
//
//    fn lookup(&self, key: &KeyType)
//        -> Box< Future<Item=ValueType, Error=StorageError> >
//    {
//        self.map.get(&key)
//            .map( |v| v.to_owned() )
//            .ok_or(StorageError::InvalidKey)
//    }
//}
//
//
//
//#[cfg(test)]
//mod tests
//{
//    use super::*;
//
//
//    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
//    struct Person
//    {
//        name:  String,
//        phone: String,
//        age:   u16,
//    }
//
//
//    #[test]
//    fn test_serialization()
//    {
//        let serializer = SerdeJsonSerializer;
//        let orig_obj = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
//        let ser_obj = serializer.serialize(&orig_obj);
//        assert!( ser_obj.is_ok() );
//        let deser_res = serializer.deserialize( &ser_obj.unwrap() );
//        assert!( deser_res.is_ok() );
//        assert_eq!( orig_obj, deser_res.unwrap() );
//    }
//
//    #[test]
//    fn test_hash()
//    {
//        let ser_obj = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
//        let hasher = MultiHasher{hash_algorithm: multihash::Hash::Keccak256};
//        let hash = hasher.hash(&ser_obj);
//        assert!( hash.is_ok() );
//        let valid = hasher.validate( &ser_obj, &hash.unwrap() );
//        assert!( valid.is_ok() );
//    }
//
//    #[test]
//    fn test_storage()
//    {
//        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
//        let hash = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
//        let mut storage: InMemoryStore<Vec<u8>,Person> = InMemoryStore::new();
//        let store_res = storage.store( &hash, object.clone() );
//        assert!( store_res.is_ok() );
//        let lookup_res = storage.lookup(&hash);
//        assert!( lookup_res.is_ok() );
//        assert_eq!( lookup_res.unwrap(), object );
//    }
//
//    #[test]
//    fn test_hashspace()
//    {
//        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
//        let mut hashspace: CompositeHashSpace<Person, Vec<u8>, Vec<u8>> = CompositeHashSpace{
//            serializer: Box::new( SerdeJsonSerializer{} ),
//            hasher:     Box::new( MultiHasher{hash_algorithm: multihash::Hash::Keccak512} ),
//            storage:    Box::new(store) };
//
//        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
//        let store_res = hashspace.store( object.clone() );
//        assert!( store_res.is_ok() );
//        let hash = store_res.unwrap();
//        let lookup_res = hashspace.lookup(&hash);
//        assert!( lookup_res.is_ok() );
//        assert_eq!( lookup_res.unwrap(), object );
//        let validate_res = hashspace.validate(&object, &hash);
//        assert!( validate_res.is_ok() );
//        assert!( validate_res.unwrap() );
//    }
//}
