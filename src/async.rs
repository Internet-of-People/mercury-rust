use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::rc::Rc;

use futures::prelude::*;
use futures::future;
use tokio_core::reactor;

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

        match hash_result {
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
KeyValueStore
for InMemoryStore<KeyType, ValueType>
    where KeyType: Eq + Hash + Clone,
          ValueType: Clone + 'static
{
    type KeyType = KeyType;
    type ValueType = ValueType;

    fn store(&mut self, key: KeyType, object: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        self.map.insert(key.to_owned(), object );
        Box::new( future::ok(() ) )
    }

    fn lookup(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >
    {
        let result = match self.map.get(&key) {
            Some(val) => future::ok( val.to_owned() ),
            None      => future::err(StorageError::InvalidKey),
        };
        Box::new(result)
    }
}



#[cfg(test)]
mod tests
{
    use std::thread;
    use std::time::Duration;

    use futures::sync::oneshot;

    use super::*;
    use super::super::*;

    use common::*;



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
        let hash = "key".to_string();
        let mut storage: InMemoryStore<String,Person> = InMemoryStore::new();
        let store_res = storage.store( hash.clone(), object.clone() ).wait();
        assert!( store_res.is_ok() );
        let lookup_res = storage.lookup(hash).wait();
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
    }


//    #[test]
//    fn test_hashspace()
//    {
//        let store: InMemoryStore<String, Vec<u8>> = InMemoryStore::new();
//        let mut hashspace: CompositeHashSpace<Person> = CompositeHashSpace{
//            serializer: Rc::new( SerdeJsonSerializer{} ),
//            hasher:     Rc::new( MultiHasher{hash_algorithm: multihash::Hash::Keccak512} ),
//            storage:    Rc::new(store) };
//
//        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
//        let store_res = hashspace.store( object.clone() ).wait();
//        assert!( store_res.is_ok() );
//        let hash = store_res.unwrap();
//        let lookup_res = hashspace.resolve( hash.clone() ).wait();
//        assert!( lookup_res.is_ok() );
//        assert_eq!( lookup_res.unwrap(), object );
//        let validate_res = hashspace.validate(&object, &hash).wait();
//        assert!( validate_res.is_ok() );
//        assert!( validate_res.unwrap() );
//    }



//    fn start_reactor_thread() -> reactor::Remote
//    {
//        // Run a separate db event loop for potentially long running blocking operations
//        let (sender, receiver) = oneshot::channel();
//
//        thread::spawn( ||
//        {
//            // TODO consider if these should also use assert!() calls instead of expect/unwrap
//            let mut reactor = reactor::Core::new()
//                .expect("Failed to initialize the reactor event loop");
//            // Leak out reactor remote handler to be able to spawn tasks for it from the server
//            sender.send( reactor.remote() ).unwrap();
//
//            let timeout = Duration::from_secs(1);
//            loop { reactor.turn( Some(timeout) ); }
//        } );
//
//        let reactor_proxy = receiver.wait()
//            .expect("Error implementing db event loop initialization");
//        reactor_proxy
//    }
}
