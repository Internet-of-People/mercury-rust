use futures::prelude::*;

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


