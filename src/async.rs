use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

use futures::prelude::*;
use futures::future;
use tokio_core::reactor;
use tokio_postgres;

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



pub struct CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    serializer: Rc< Serializer<ObjectType, SerializedType> >,
    hasher:     Box< Hasher<SerializedType, HashType> >,
    storage:    Box< KeyValueStore<HashType, SerializedType> >,
}


impl<ObjectType: 'static, SerializedType: 'static, HashType: Clone + 'static>
HashSpace<ObjectType, HashType>
for CompositeHashSpace<ObjectType, SerializedType, HashType>
{
    fn store(&mut self, object: ObjectType)
        -> Box< Future<Item=HashType, Error=HashSpaceError> >
    {
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

    fn resolve(&self, hash: HashType)
        -> Box< Future<Item=ObjectType, Error=HashSpaceError> >
    {
        let serializer_clone = self.serializer.clone();
        let result = self.storage.lookup(hash)
            .map_err( |e| HashSpaceError::StorageError(e) )
            .and_then( move |serialized_obj|
                serializer_clone.deserialize(&serialized_obj)
                    .map_err( move |e| HashSpaceError::SerializerError(e) ) );
        Box::new(result)
    }

    fn validate(&self, object: &ObjectType, hash: &HashType)
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
    pub fn new() -> Self { InMemoryStore{ map: HashMap::new() } }
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



struct PostgresStore
{
    reactor_handle: reactor::Handle,
    postgres_url:   String,
}

impl PostgresStore
{
    fn new(reactor_handle: &reactor::Handle, postgres_url: &str) -> Self
    {
        Self{ reactor_handle: reactor_handle.clone(),
              postgres_url:   postgres_url.to_string() }
    }

    fn connect(&self) -> Box< Future<Item=tokio_postgres::Connection, Error=tokio_postgres::Error> >
    {
        tokio_postgres::Connection::connect( self.postgres_url.as_str(),
            tokio_postgres::TlsMode::None, &self.reactor_handle)
    }
}

impl KeyValueStore<String, Vec<u8>> for PostgresStore
{
    fn store(&mut self, key: String, object: Vec<u8>)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        let result = self.connect()
            .then( |conn| {
                conn.expect("Connection to database failed")
                    .prepare("SELECT todo FROM todo") // TODO implement SQL query
            } )
            .map( |_| () )
            .map_err( |(e,c)| StorageError::Other( Box::new(e) ) );
        Box::new(result)
    }

    fn lookup(&self, key: String)
        -> Box< Future<Item=Vec<u8>, Error=StorageError> >
    {
        //TODO
        Box::new( future::err(StorageError::InvalidKey) )
    }
}



#[cfg(test)]
mod tests
{
//    use std::thread;
//    use std::time::Duration;

//    use futures::sync::oneshot;
//    use tokio_core::reactor;

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
        // NOTE this works without a tokio::reactor::Core only because
        //      the storage always returns an already completed future::ok/err result
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
        // NOTE this works without a tokio::reactor::Core only because
        //      all plugins always return an already completed ok/err result
        let store: InMemoryStore<String, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: CompositeHashSpace<Person, Vec<u8>, String> = CompositeHashSpace{
            serializer: Rc::new( SerdeJsonSerializer{} ),
            hasher:     Box::new( MultiHasher::new(multihash::Hash::Keccak512) ),
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
