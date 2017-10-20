use std::collections::HashMap;
use std::hash::Hash;

use futures::prelude::*;
use futures::future;
//use futures::future::IntoFuture;

use common::*;
use error::*;


pub trait HashSpace<ObjectType, HashType>
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=HashType, Error=HashSpaceError> >;
    fn resolve(&self, hash: HashType)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >;
    fn validate(&self, object: &ObjectType, hash: &HashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >;
}



pub trait KeyValueStore<KeyType, ValueType>
{
    fn store(&mut self, key: KeyType, object: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn lookup(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >;
}



//pub struct CompositeHashSpace<ObjectType, SerializedType, HashType>
//{
//    serializer: Box< Serializer<ObjectType, SerializedType> >,
//    hasher:     Box< Hasher<SerializedType, HashType> >,
//    storage:    Box< KeyValueStore<HashType, SerializedType> >,
//}
//
//
//impl <ObjectType, SerializedType, HashType>
//HashSpace<ObjectType, HashType>
//for CompositeHashSpace<ObjectType, SerializedType, HashType>
//    where HashType: ToOwned
//{
//    fn store(&mut self, object: ObjectType)
//        -> Box< Future<Item=HashType, Error=HashSpaceError> >
//    {
//        let hash_result: Result<(SerializedType,HashType), HashSpaceError> = self.serializer.serialize(&object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )
//            .and_then( |serialized_obj|
//                self.hasher.hash(&serialized_obj)
//                    .map( |obj_hash| (serialized_obj, obj_hash) )
//                    .map_err( |e| HashSpaceError::HashError(e) )
//            );
//        match hash_result {
//            Err(e) => Box::new( future::err(e) ),
//            Ok( (serialized_obj, obj_hash) ) => {
//                let result = self.storage.store( obj_hash.to_owned(), serialized_obj )
//                    .map( |_| obj_hash )
//                    .map_err( |e| HashSpaceError::StorageError(e) );
//                Box::new(result)
//            }
//        }
//    }
//
//    fn resolve(&self, hash: HashType)
//        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >
//    {
//        let result = self.storage.lookup(hash)
//            .map_err( |e| HashSpaceError::StorageError(e) )
//            .and_then( |serialized_obj|
//                self.serializer.deserialize(&serialized_obj)
//                    .map_err( |e| HashSpaceError::SerializerError(e) ) );
//        Box::new(result)
//    }
//
//    fn validate(&self, object: &ObjectType, hash: &HashType)
//        -> Box< Future<Item=bool, Error=HashSpaceError> >
//    {
//        let valid = self.serializer.serialize(&object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )
//            .and_then( |serialized_obj|
//                self.hasher.validate(&serialized_obj, &hash)
//                    .map_err( |e| HashSpaceError::HashError(e) ) );
//        Box::new( future::res(valid) )
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
//        { InMemoryStore{ map: HashMap::new() } }
//}
//
//impl<KeyType, ValueType>
//KeyValueStore<KeyType, ValueType>
//for InMemoryStore<KeyType, ValueType>
//    where KeyType: Eq + Hash + Clone,
//          ValueType: Clone
//{
//    fn store(&mut self, key: KeyType, object: ValueType)
//        -> Box< Future<Item=(), Error=StorageError> >
//    {
//        self.map.insert(key.to_owned(), object );
//        //Box::new( futures::future::ok(true) )
//    }
//
//    fn lookup(&self, key: KeyType)
//        -> Box< Future<Item=ValueType, Error=StorageError> >
//    {
//        self.map.get(&key)
////        let result = match self.map.get(&key) {
////            Ok(val) => futures::future::ok( val.to_owned() ),
////            Err(e)  => futures::future::error(StorageError::InvalidKey),
////        };
////        Box::new(result)
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
