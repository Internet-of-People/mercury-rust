use std::rc::Rc;

use futures::prelude::*;
use futures::future;

use common::*;
use error::*;

pub mod imp;



pub trait HashSpace<ObjectType>
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=String, Error=HashSpaceError> >;
    fn resolve(&self, hash: &str)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >;
    fn validate(&self, object: &ObjectType, hash: &str)
        -> Box< Future<Item=bool, Error=HashSpaceError> >;
}


pub trait KeyValueStore<KeyType, ValueType>
{
    // TODO maybe it would be enough to use references instead of consuming params
    fn store(&mut self, key: KeyType, value: ValueType)
        -> Box< Future<Item=(), Error=StorageError> >;
    fn lookup(&self, key: KeyType)
        -> Box< Future<Item=ValueType, Error=StorageError> >;
}



pub struct CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    serializer: Rc< Serializer<ObjectType, SerializedType> >,
    hasher:     Rc< Hasher<SerializedType, HashType> >,
    storage:    Box< KeyValueStore<HashType, SerializedType> >,
    str_coder:  Box< StringCoder<HashType> >,
}


impl<ObjectType, SerializedType, HashType>
CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    pub fn new( serializer: Rc< Serializer<ObjectType, SerializedType> >,
                hasher:     Rc< Hasher<SerializedType, HashType> >,
                storage:    Box< KeyValueStore<HashType, SerializedType> >,
                str_coder:  Box< StringCoder<HashType> > ) -> Self
    {
        Self{ serializer:   serializer,
            hasher:       hasher,
            storage:      storage,
            str_coder:    str_coder, }
    }

    fn sync_validate(&self, object: &ObjectType, hash_str: &str)
        -> Result<bool, HashSpaceError>
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


impl<ObjectType, SerializedType, HashType>
HashSpace<ObjectType>
for CompositeHashSpace<ObjectType, SerializedType, HashType>
    where ObjectType: 'static,
          SerializedType: 'static,
          HashType: 'static + Clone
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=String, Error=HashSpaceError> >
    {
        let hash_bytes_result = self.serializer.serialize(&object)
            .map_err( |e| HashSpaceError::SerializerError(e) )
            .and_then( |serialized_obj|
                self.hasher.get_hash(&serialized_obj)
                    .map( |obj_hash| (serialized_obj, obj_hash) )
                    .map_err( |e| HashSpaceError::HashError(e) )
            );

        if let Err(e) = hash_bytes_result
            { return Box::new( future::err(e) ); }
        let (serialized_obj, hash_bytes) = hash_bytes_result.unwrap();

        let hash_str_result = self.str_coder.encode(&hash_bytes)
            .map_err( |e| HashSpaceError::StringCoderError(e) );
        if let Err(e) = hash_str_result
            { return Box::new( future::err(e) ); }
        let hash_str = hash_str_result.unwrap();

        let result = self.storage.store( hash_bytes, serialized_obj )
            .map( |_| hash_str )
            .map_err( |e| HashSpaceError::StorageError(e) );
        Box::new(result)
    }

    fn resolve(&self, hash_str: &str)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >
    {
        let hash_bytes_result = self.str_coder.decode(&hash_str)
            .map_err( |e| HashSpaceError::StringCoderError(e) );
        let hash_bytes = match hash_bytes_result {
            Err(e)  => return Box::new( future::err(e) ),
            Ok(val) => val,
        };

        let hash_bytes_clone = hash_bytes.clone();
        let hasher_clone = self.hasher.clone();
        let serializer_clone = self.serializer.clone();
        let result = self.storage.lookup(hash_bytes)
            .map_err( |e| HashSpaceError::StorageError(e) )
            .and_then( move |serialized_obj|
                match hasher_clone.validate(&serialized_obj, &hash_bytes_clone) {
                    Err(e) => Err( HashSpaceError::HashError(e) ),
                    Ok(v)  => if v { Ok(serialized_obj) }
                        // TODO consider using a different error code
                        else { Err( HashSpaceError::StorageError(StorageError::InvalidKey) ) }
                } )
            .and_then( move |serialized_obj|
                serializer_clone.deserialize(&serialized_obj)
                    .map_err(  |e| HashSpaceError::SerializerError(e) ) );
        Box::new(result)
    }

    fn validate(&self, object: &ObjectType, hash_str: &str)
        -> Box< Future<Item=bool, Error=HashSpaceError> >
    {
        Box::new( future::result(self.sync_validate(&object, &hash_str) ) )
    }
}
