use std::collections::HashMap;
use std::hash::Hash;
use std::io;
use std::marker;
use std::rc::Rc;

use base64;
use futures::prelude::*;
use futures::future;
use multihash;
use serde;
use serde_json;
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


pub trait Serializer<ObjectType, SerializedType>
{
    // TODO error handling: these two operations could return different error types
    //      (SerErr/DeserErr), consider if that might be clearer
    fn serialize(&self, object: &ObjectType)
        -> Result<SerializedType, SerializerError>;
    fn deserialize(&self, serialized_object: &SerializedType)
        -> Result<ObjectType, SerializerError>;
}

pub trait Hasher<SerializedType, HashType>
{
    fn get_hash(&self, serialized_data: &SerializedType)
        -> Result<HashType, HashError>;
    fn validate(&self, serialized_data: &SerializedType, hash: &HashType)
        -> Result<bool, HashError>;
}

pub trait KeyValueStore<KeyType, ValueType>
{
    fn store(&mut self, key: KeyType, object: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn lookup(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >;
}



pub struct CompositeHashSpace<ObjectType>
{
    serializer: Rc< Serializer<ObjectType, DefaultSerializedType> >,
    hasher:     Box< Hasher<DefaultSerializedType, DefaultHashType> >,
    storage:    Box< KeyValueStore<DefaultHashType, DefaultSerializedType> >,
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
//        let mut storage_rc_clone = self.storage.clone();
//        let storage_opt = Rc::get_mut(&mut storage_rc_clone);
//        let storage_res = storage_opt.ok_or( HashSpaceError::Other(
//            Box::new( io::Error::new(io::ErrorKind::PermissionDenied, "Implementation error: could not get access to Rc") ) ) );
//        let storage = match storage_res {
//            Err(e)  => return Box::new( future::err(e) ),
//            Ok(val) => val,
//        };

        let hash_result = self.serializer.serialize(&object)
            .map_err( |e| HashSpaceError::SerializerError(e) )
            .and_then( |serialized_obj|
                self.hasher.get_hash(&serialized_obj)
                    .map( |obj_hash| (serialized_obj, obj_hash) )
                    .map_err( |e| HashSpaceError::HashError(e) )
            );

        if let Err(e) = hash_result
            { return Box::new( future::err(e) ); }
        let (serialized_obj, obj_hash) = hash_result.unwrap();

        let result = self.storage.store( obj_hash.clone(), serialized_obj )
            .map( |_| obj_hash )
            .map_err( |e| HashSpaceError::StorageError(e) );
        Box::new(result)
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



// TODO this struct should be independent of the serialization format (e.g. JSON):
//      Maybe should contain Box<serde::ser::De/Serializer> data members
pub struct SerdeJsonSerializer<Obj>
{
    _phantom: marker::PhantomData<Obj>,
}

impl<Obj> SerdeJsonSerializer<Obj>
{
    pub fn new() -> Self { Self{_phantom: marker::PhantomData{} } }

    fn to_serializer_error(error: serde_json::Error) -> SerializerError
        { SerializerError::SerializationError( Box::new(error) ) }
}

impl<ObjectType: serde::Serialize + serde::de::DeserializeOwned>
Serializer<ObjectType, DefaultSerializedType> for SerdeJsonSerializer<ObjectType>
{
    fn serialize(&self, object: &ObjectType)
        -> Result<DefaultSerializedType, SerializerError>
    {
        serde_json::to_string(&object)
            .map( |str| str.into_bytes() )
            .map_err(SerdeJsonSerializer::<ObjectType>::to_serializer_error)
    }

    fn deserialize(&self, serialized_object: &DefaultSerializedType)
        -> Result<ObjectType, SerializerError>
    {
        let json_string = String::from_utf8(serialized_object.clone() )
            .map_err(|e| SerializerError::DeserializationError( Box::new(e) ) )?;
        serde_json::from_str(& json_string)
            .map_err(SerdeJsonSerializer::<ObjectType>::to_serializer_error)
    }
}



pub struct MultiHasher
{
    hash_algorithm: multihash::Hash,
}

impl MultiHasher
{
    pub fn new(hash_algorithm: multihash::Hash) -> Self
        { MultiHasher{hash_algorithm: hash_algorithm} }

    fn to_hasher_error(error: multihash::Error) -> HashError
    {
        match error {
            multihash::Error::BadInputLength    => HashError::BadInputLength,
            multihash::Error::UnkownCode        => HashError::UnknownCode,
            multihash::Error::UnsupportedType   => HashError::UnsupportedType,
        }
    }

    fn get_hash_bytes(&self, data: &Vec<u8>) -> Result<Vec<u8>, HashError>
    {
        multihash::encode(self.hash_algorithm, data)
            .map_err(MultiHasher::to_hasher_error)
    }

    fn get_hash_string(&self, data: &Vec<u8>) -> Result<String, HashError>
    {
        self.get_hash_bytes(&data)
            // TODO this should use something like a "multibase" lib, similar to multihash
            .map( |bytes| base64::encode(&bytes) )
    }
}


impl Hasher<DefaultSerializedType, DefaultHashType> for MultiHasher
{
    fn get_hash(&self, serialized_data: &DefaultSerializedType)
            -> Result<DefaultHashType, HashError>
        { self.get_hash_string(&serialized_data) }

    fn validate(&self, serialized_data: &DefaultSerializedType, expected_hash: &DefaultHashType)
        -> Result<bool, HashError>
    {
        //        // TODO should we do this here or just drop this step and check hash equality?
        //        let decode_result = decode(expected_hash)
        //            .map_err(MultiHasher::to_hasher_error)?;
        //        if decode_result.alg != self.hash_algorithm
        //            { return Err(HashError::UnsupportedType); }

        let calculated_hash = self.get_hash_string(&serialized_data)?;
        Ok(*expected_hash == calculated_hash)
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
          ValueType: Clone + 'static
{
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



    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person
    {
        name:  String,
        phone: String,
        age:   u16,
    }


    #[test]
    fn test_serializer()
    {
        let serializer = SerdeJsonSerializer::new();
        let orig_obj = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let ser_obj = serializer.serialize(&orig_obj);
        assert!( ser_obj.is_ok() );
        let deser_res = serializer.deserialize( &ser_obj.unwrap() );
        assert!( deser_res.is_ok() );
        assert_eq!( orig_obj, deser_res.unwrap() );
    }


    #[test]
    fn test_hasher()
    {
        let ser_obj = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let hasher = MultiHasher{hash_algorithm: multihash::Hash::Keccak256};
        let hash = hasher.get_hash(&ser_obj);
        assert!( hash.is_ok() );
        let valid = hasher.validate( &ser_obj, &hash.unwrap() );
        assert!( valid.is_ok() );
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


    #[test]
    fn test_hashspace()
    {
        let store: InMemoryStore<String, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: CompositeHashSpace<Person> = CompositeHashSpace{
            serializer: Rc::new( SerdeJsonSerializer::new() ),
            hasher:     Box::new( MultiHasher{hash_algorithm: multihash::Hash::Keccak512} ),
            storage:    Box::new(store) };

        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let store_res = hashspace.store( object.clone() ).wait();
        assert!( store_res.is_ok() );
        let hash = store_res.unwrap();
        let lookup_res = hashspace.resolve( hash.clone() ).wait();
        assert!( lookup_res.is_ok() );
        assert_eq!( lookup_res.unwrap(), object );
        let validate_res = hashspace.validate(&object, &hash).wait();
        assert!( validate_res.is_ok() );
        assert!( validate_res.unwrap() );
    }



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
