use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::rc::Rc;

use futures::prelude::*;
use futures::future;

use error::*;


type DefaultSerializedType = Vec<u8>;
type DefaultHashType = String;



pub trait HashSpace
{
    type ObjectType;
    type HashType; // = DefaultHashType;

    fn store(&mut self, object: Self::ObjectType)
        -> Box< Future<Item=Self::HashType, Error=HashSpaceError> >;
    fn resolve(&self, hash: Self::HashType)
        -> Box< Future<Item=Self::ObjectType, Error=HashSpaceError> >;
    fn validate(&self, object: &Self::ObjectType, hash: &Self::HashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >;
}


pub trait Serializer
{
    type ObjectType;
    type SerializedType; // = DefaultSerializedType;

    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: &Self::ObjectType)
        -> Result<Self::SerializedType, SerializerError>;
    fn deserialize(&self, serialized_object: &Self::SerializedType)
        -> Result<Self::ObjectType, SerializerError>;
}

pub trait Hasher
{
    type SerializedType;
    type HashType; // = DefaultHashType;

    fn get_hash(&self, object: &Self::SerializedType)
        -> Result<Self::HashType, HashError>;
    fn validate(&self, object: &Self::SerializedType, hash: &Self::HashType)
        -> Result<bool, HashError>;
}

pub trait KeyValueStore
{
    type KeyType; // = DefaultHashType;
    type ValueType; // = DefaultSerializedType;

    fn store(&mut self, key: Self::KeyType, object: Self::ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn lookup(&self, key: Self::KeyType)
        -> Box< Future<Item=Self::ValueType, Error=StorageError> >;
}



pub struct CompositeHashSpace<Obj>
{
    serializer: Rc< Serializer<ObjectType=Obj, SerializedType=DefaultSerializedType> >,
    hasher:     Rc< Hasher<SerializedType=DefaultSerializedType, HashType=DefaultHashType> >,
    storage:    Rc< KeyValueStore<KeyType=DefaultHashType, ValueType=DefaultSerializedType> >,
}



impl<Obj: 'static>
HashSpace
for CompositeHashSpace<Obj>
{
    type ObjectType = Obj;
    type HashType = DefaultHashType;

    fn store(&mut self, object: Self::ObjectType)
        -> Box< Future<Item=Self::HashType, Error=HashSpaceError> >
    {
        let mut storage_rc_clone = self.storage.clone();
        let storage_opt = Rc::get_mut(&mut storage_rc_clone);
        let storage_res = storage_opt.ok_or( HashSpaceError::Other(
            Box::new( io::Error::new(io::ErrorKind::PermissionDenied, "Implementation error: could not get access to Rc") ) ) );
        let storage = match storage_res {
            Err(e)  => return Box::new( future::err(e) ),
            Ok(val) => val,
        };

        let hash_result = self.serializer.serialize(&object)
            .map_err( |e| HashSpaceError::SerializerError(e) )
            .and_then( |serialized_obj|
                self.hasher.get_hash(&serialized_obj)
                    .map( |obj_hash| (serialized_obj, obj_hash) )
                    .map_err( |e| HashSpaceError::HashError(e) )
            );

        match hash_result
        {
            Err(e) => Box::new( future::err(e) ),
            Ok( (serialized_obj, obj_hash) ) => {
                Box::new( storage.store( obj_hash.clone(), serialized_obj )
                    .map( |_| obj_hash )
                    .map_err( |e| HashSpaceError::StorageError(e) ) )
            }
        }
    }

    fn resolve(&self, hash: Self::HashType)
        -> Box< Future<Item=Self::ObjectType, Error=HashSpaceError> >
    {
        let serializer_clone = self.serializer.clone();
        let result = self.storage.lookup(hash)
            .map_err( |e| HashSpaceError::StorageError(e) )
            .and_then( move |serialized_obj|
                serializer_clone.deserialize(&serialized_obj)
                    .map_err( move |e| HashSpaceError::SerializerError(e) ) );
        Box::new(result)
    }

    fn validate(&self, object: &Self::ObjectType, hash: &Self::HashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >
    {
        let valid = self.serializer.serialize(&object)
            .map_err( |e| HashSpaceError::SerializerError(e) )
            .and_then( |serialized_obj|
                self.hasher.validate(&serialized_obj, &hash)
                    .map_err( |e| HashSpaceError::HashError(e) ) );
        Box::new( future::result(valid) )
    }
}



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
