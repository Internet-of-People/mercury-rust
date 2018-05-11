#![allow(unused, non_snake_case, non_upper_case_globals)]

use std::rc::Rc;

use futures::prelude::*;
use futures::future;

use common::*;
use error::*;

pub mod imp;



pub trait HashSpace<ObjectType, ReadableHashType>
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=ReadableHashType, Error=HashSpaceError> >;
    fn resolve(&self, hash: &ReadableHashType)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >;
    fn validate(&self, object: &ObjectType, hash: &ReadableHashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >;
}


pub trait KeyValueStore<KeyType, ValueType>
{
    // TODO maybe it would be enough to use references instead of consuming params
    fn set(&mut self, key: KeyType, value: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn get(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >;
}



pub struct ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
//    serializer: Rc< Serializer<ObjectType, SerializedType> >,
    hasher:     Rc< Hasher<SerializedType, BinaryHashType> >,
    storage:    Box< KeyValueStore<BinaryHashType, SerializedType> >,
    hash_coder: Box< HashCoder<BinaryHashType, ReadableHashType> >,
}


impl<SerializedType, BinaryHashType, ReadableHashType>
ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
    pub fn new( // serializer: Rc< Serializer<ObjectType, SerializedType> >,
                hasher:     Rc< Hasher<SerializedType, BinaryHashType> >,
                storage:    Box< KeyValueStore<BinaryHashType, SerializedType> >,
                hash_coder: Box< HashCoder<BinaryHashType, ReadableHashType> > ) -> Self
    {
        Self{ // serializer:   serializer,
              hasher:       hasher,
              storage:      storage,
              hash_coder:   hash_coder, }
    }

    fn sync_validate(&self, serialized_obj: &SerializedType, readable_hash: &ReadableHashType)
        -> Result<bool, HashSpaceError>
    {
        let hash_bytes = self.hash_coder.decode(readable_hash)
            .map_err( |e| HashSpaceError::StringCoderError(e) )?;
//        let serialized_obj = self.serializer.serialize(object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )?;
        let valid = self.hasher.validate(&serialized_obj, &hash_bytes)
            .map_err( |e| HashSpaceError::HashError(e) )?;
        Ok(valid)
    }
}


impl<SerializedType, BinaryHashType, ReadableHashType>
HashSpace<SerializedType, ReadableHashType>
for ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
    where // ObjectType: 'static,
          SerializedType: 'static,
          BinaryHashType: 'static + Clone,
          ReadableHashType: 'static
{
    fn store(&mut self, serialized_obj: SerializedType)
        -> Box< Future<Item=ReadableHashType, Error=HashSpaceError> >
    {
//        let hash_bytes_result = self.serializer.serialize(object)
//            .map_err( |e| HashSpaceError::SerializerError(e) )
//            .and_then( |serialized_obj|
//                self.hasher.get_hash(&serialized_obj)
//                    .map( |obj_hash| (serialized_obj, obj_hash) )
//                    .map_err( |e| HashSpaceError::HashError(e) )
//            );
        let hash_bytes_result = self.hasher.get_hash(&serialized_obj)
            .map( |obj_hash| (serialized_obj, obj_hash) )
            .map_err( |e| HashSpaceError::HashError(e) );

        if let Err(e) = hash_bytes_result
            { return Box::new( future::err(e) ); }
        let (serialized_obj, hash_bytes) = hash_bytes_result.unwrap();

        let hash_str_result = self.hash_coder.encode(&hash_bytes)
            .map_err( |e| HashSpaceError::StringCoderError(e) );
        if let Err(e) = hash_str_result
            { return Box::new( future::err(e) ); }
        let hash_str = hash_str_result.unwrap();

        let result = self.storage.set(hash_bytes, serialized_obj )
            .map( |_| hash_str )
            .map_err( |e| HashSpaceError::StorageError(e) );
        Box::new(result)
    }


    fn resolve(&self, hash_str: &ReadableHashType)
        -> Box< Future<Item=SerializedType, Error=HashSpaceError> >
    {
        let hash_bytes_result = self.hash_coder.decode(&hash_str)
            .map_err( |e| HashSpaceError::StringCoderError(e) );
        let hash_bytes = match hash_bytes_result {
            Err(e)  => return Box::new( future::err(e) ),
            Ok(val) => val,
        };

        let hash_bytes_clone = hash_bytes.clone();
        let hasher_clone = self.hasher.clone();
        // let serializer_clone = self.serializer.clone();
        let result = self.storage.get(hash_bytes)
            .map_err( |e| HashSpaceError::StorageError(e) )
            .and_then( move |serialized_obj|
                match hasher_clone.validate(&serialized_obj, &hash_bytes_clone) {
                    Err(e) => Err( HashSpaceError::HashError(e) ),
                    Ok(v)  => if v { Ok(serialized_obj) }
                        // TODO consider using a different error code
                        else { Err( HashSpaceError::StorageError(StorageError::InvalidKey) ) }
                } );
//            .and_then( move |serialized_obj|
//                serializer_clone.deserialize(serialized_obj)
//                    .map_err(  |e| HashSpaceError::SerializerError(e) ) );
        Box::new(result)
    }

    fn validate(&self, object: &SerializedType, hash_str: &ReadableHashType)
        -> Box< Future<Item=bool, Error=HashSpaceError> >
    {
        Box::new( future::result( self.sync_validate( &object, &hash_str) ) )
    }
}
