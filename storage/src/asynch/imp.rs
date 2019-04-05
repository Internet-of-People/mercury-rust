#![allow(unused, non_snake_case)]

use std::collections::HashMap;
use std::error::Error;
use std::hash::Hash;
use std::rc::Rc;

use futures::future;
use futures::prelude::*;
use futures_state_stream::StateStream;
use serde_derive::{Deserialize, Serialize};
use tokio_core::reactor;

use crate::asynch::*;

const HashWebLink_HashSpaceId_Separator: &str = "/";
const HashWebLink_Attribute_Separator: &str = "#";

pub type HashSpaceId = String;

//pub trait HashLink
//{
//    fn hashspace(&self) -> &HashSpaceId;
//    fn hash(&self)      -> &str;          // of linked data under specified hashspace
//}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HashWebLink {
    hashspace: HashSpaceId,
    hash: String,
}

impl HashWebLink {
    // TODO solve using &str instead of &String
    pub fn new(hashspace: &HashSpaceId, hash: &str) -> Self {
        Self { hashspace: hashspace.to_owned(), hash: hash.to_owned() }
    }

    pub fn hashspace(&self) -> &HashSpaceId {
        &self.hashspace
    }
    pub fn hash(&self) -> &str {
        self.hash.as_ref()
    }

    pub fn parse(address_str: &str) -> Result<HashWebLink, HashSpaceError> {
        // Ignore starting slash
        let address = if address_str.starts_with('/') { &address_str[1..] } else { address_str };

        // Split hashspaceId and hash parts
        let slash_pos =
            address.find('/').ok_or(HashSpaceError::LinkFormatError(address_str.to_owned()))?; //.unwrap_or( address.len() );
        let (hashspace_id, slashed_hash) = address.split_at(slash_pos);
        let hash = &slashed_hash[1..]; // Ignore starting slash

        // Perform link resolution
        let hashlink = HashWebLink::new(&hashspace_id.to_string(), hash);
        Ok(hashlink)
    }
}

pub struct HashWeb<ObjectType> {
    hashspaces: HashMap<HashSpaceId, Box<HashSpace<ObjectType, String>>>,
    default: HashSpaceId,
}

impl<ObjectType: 'static> HashWeb<ObjectType> {
    pub fn new(
        hashspaces: HashMap<HashSpaceId, Box<HashSpace<ObjectType, String>>>,
        default: HashSpaceId,
    ) -> Self {
        HashWeb { hashspaces, default }
    }

    //    // Expected hashlink format: hashspaceId/hash
    //    pub fn resolve_hashlink(&self, hashlink_str: &str)
    //        -> Box< Future<Item=ObjectType, Error=AddressResolutionError> >
    //    {
    //
    //        let hashlink = match HashWebLink::parse(hashlink_str) {
    //            Ok(link) => link,
    //            Err(e) => return Box::new( future::err(AddressResolutionError::HashSpaceError(e) ) ),
    //        };
    //        let resolved_data_fut = self.resolve(&hashlink)
    //            .map_err( |e| AddressResolutionError::HashSpaceError(e) );
    //        Box::new(resolved_data_fut)
    //    }
}

// TODO this implementation is very similar to HashSpace<ObjectType, String>,
//      most code should be shared between them if both are needed
//impl<ObjectType>
//HashSpace<ObjectType, HashWebLink>
//for HashWeb<ObjectType>
//where ObjectType: 'static
//{
//    fn store(&mut self, object: ObjectType)
//         -> Box< Future<Item=HashWebLink, Error=HashSpaceError> >
//    {
//        let mut hashspace_res = self.hashspaces.get_mut(&self.default)
//            .ok_or( HashSpaceError::UnsupportedHashSpace( self.default.to_owned() ) );;
//        let hashspace = match hashspace_res {
//            Ok(ref mut space) => space,
//            Err(e) => return Box::new( future::err(e) ),
//        };
//        let default_hashspace_clone = self.default.clone();
//        let result = hashspace.store(object)
//            .map( move |hash| HashWebLink::new(&default_hashspace_clone, &hash) );
//        Box::new(result)
//    }
//
//
//    fn resolve(&self, link: &HashWebLink)
//        -> Box< Future<Item = ObjectType, Error = HashSpaceError> >
//    {
//        let hashspace_res = self.hashspaces.get( link.hashspace() )
//            .ok_or( HashSpaceError::UnsupportedHashSpace( link.hashspace().to_owned() ) );
//        let hashspace = match hashspace_res {
//            Ok(space) => space,
//            Err(e) => return Box::new( future::err(e) ),
//        };
//        let data = hashspace.resolve( &link.hash().to_owned() );
//        Box::new(data)
//    }
//
//
//    fn validate(&self, object: &ObjectType, link: &HashWebLink)
//        -> Box< Future<Item=bool, Error=HashSpaceError> >
//    {
//        let hashspace_res = self.hashspaces.get( link.hashspace() )
//            .ok_or( HashSpaceError::UnsupportedHashSpace( link.hashspace().to_owned() ) );
//        let hashspace = match hashspace_res {
//            Ok(ref space) => space,
//            Err(e) => return Box::new( future::err(e) ),
//        };
//        // TODO to_string() is unnecessary below, find out how to transform signatures so as it's not needed
//        let result = hashspace.validate( object, &link.hash().to_string() );
//        Box::new(result)
//    }
//}

impl<ObjectType> HashSpace<ObjectType, String> for HashWeb<ObjectType>
where
    ObjectType: 'static,
{
    fn store(&mut self, object: ObjectType) -> Box<Future<Item = String, Error = HashSpaceError>> {
        let mut hashspace_res = self
            .hashspaces
            .get_mut(&self.default)
            .ok_or(HashSpaceError::UnsupportedHashSpace(self.default.to_owned()));;
        let hashspace = match hashspace_res {
            Ok(ref mut space) => space,
            Err(e) => return Box::new(future::err(e)),
        };
        let default_hashspace_clone = self.default.clone();
        let result = hashspace
            .store(object)
            .map(move |hash| default_hashspace_clone + HashWebLink_HashSpaceId_Separator + &hash);
        Box::new(result)
    }

    fn resolve(
        &self,
        hashlink_str: &String,
    ) -> Box<Future<Item = ObjectType, Error = HashSpaceError>> {
        let hashlink = match HashWebLink::parse(hashlink_str) {
            Ok(link) => link,
            Err(e) => return Box::new(future::err(e)),
        };

        let hashspace_res = self
            .hashspaces
            .get(hashlink.hashspace())
            .ok_or(HashSpaceError::UnsupportedHashSpace(hashlink.hashspace().to_owned()));
        let hashspace = match hashspace_res {
            Ok(space) => space,
            Err(e) => return Box::new(future::err(e)),
        };
        let data = hashspace.resolve(&hashlink.hash().to_owned());
        Box::new(data)
    }

    fn validate(
        &self,
        object: &ObjectType,
        hashlink_str: &String,
    ) -> Box<Future<Item = bool, Error = HashSpaceError>> {
        let hashlink = match HashWebLink::parse(hashlink_str) {
            Ok(link) => link,
            Err(e) => return Box::new(future::err(e)),
        };

        let hashspace_res = self
            .hashspaces
            .get(hashlink.hashspace())
            .ok_or(HashSpaceError::UnsupportedHashSpace(hashlink.hashspace().to_owned()));
        let hashspace = match hashspace_res {
            Ok(ref space) => space,
            Err(e) => return Box::new(future::err(e)),
        };
        // TODO to_string() is unnecessary below, find out how to transform signatures so as it's not needed
        let result = hashspace.validate(object, &hashlink.hash().to_string());
        Box::new(result)
    }
}

pub struct InMemoryStore<KeyType, ValueType> {
    map: HashMap<KeyType, ValueType>,
}

impl<KeyType, ValueType> InMemoryStore<KeyType, ValueType>
where
    KeyType: Eq + Hash,
{
    pub fn new() -> Self {
        InMemoryStore { map: HashMap::new() }
    }
}

impl<KeyType, ValueType> KeyValueStore<KeyType, ValueType> for InMemoryStore<KeyType, ValueType>
where
    KeyType: Eq + Hash,
    ValueType: Clone + Send + 'static,
{
    fn set(&mut self, key: KeyType, object: ValueType) -> StorageResult<()> {
        self.map.insert(key, object);
        Box::new(Ok(()).into_future())
    }

    fn get(&self, key: KeyType) -> StorageResult<ValueType> {
        let result = match self.map.get(&key) {
            Some(val) => Ok(val.to_owned()),
            None => Err(StorageError::InvalidKey),
        };
        Box::new(result.into_future())
    }

    fn clear_local(&mut self, key: KeyType) -> StorageResult<()> {
        let result = self.map.remove(&key).map(|_| ()).ok_or(StorageError::InvalidKey);
        Box::new(result.into_future())
    }
}
#[cfg(test)]
mod tests {
    use multihash;
    use tokio_core::reactor;

    use super::*;
    use crate::common::imp::*;
    use crate::meta::tests::{MetaAttr, MetaAttrVal, MetaData};
    use crate::meta::Attribute;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person {
        name: String,
        phone: String,
        age: u16,
    }

    #[test]
    fn test_inmemory_storage() {
        // NOTE this works without a tokio::reactor::Core only because
        //      the storage always returns an already completed future::ok/err result
        let object =
            Person { name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let hash = "key".to_string();
        let mut storage: InMemoryStore<String, Person> = InMemoryStore::new();
        let store_res = storage.set(hash.clone(), object.clone()).wait();
        assert!(store_res.is_ok());
        let lookup_res = storage.get(hash).wait();
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
    }

    #[test]
    fn test_hashspace() {
        // NOTE this works without a tokio::reactor::Core only because
        //      all plugins always return an already completed ok/err result
        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
            //            Rc::new( SerdeJsonSerializer{} ),
            Rc::new(MultiHasher::new(multihash::Hash::Keccak512)),
            Box::new(store),
            Box::new(MultiBaseHashCoder::new(multibase::Base64)),
        );

        //        let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let object = b"What do you get if you multiply six by nine?".to_vec();
        let store_res = hashspace.store(object.clone()).wait();
        assert!(store_res.is_ok());
        let hash = store_res.unwrap();
        let lookup_res = hashspace.resolve(&hash).wait();
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
        let validate_res = hashspace.validate(&object, &hash).wait();
        assert!(validate_res.is_ok());
        assert!(validate_res.unwrap());
    }

    #[test]
    fn test_hashweb() {
        let cache_store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let cache_space: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
            //            Rc::new( IdentitySerializer{} ),
            Rc::new(MultiHasher::new(multihash::Hash::Keccak512)),
            Box::new(cache_store),
            Box::new(MultiBaseHashCoder::new(multibase::Base64)),
        );

        let mut reactor =
            reactor::Core::new().expect("Failed to initialize the reactor event loop");

        let default_space = "cache".to_owned();
        let mut spaces: HashMap<String, Box<HashSpace<Vec<u8>, String>>> = HashMap::new();
        spaces.insert(default_space.clone(), Box::new(cache_space));
        //        spaces.insert( "postgres".to_owned(), Box::new(postgres_space) );
        let mut hashweb = HashWeb::new(spaces, default_space.clone());

        let content = b"There's over a dozen netrunners Netwatch Cops would love to brain burn and Rache Bartmoss is at least two of them".to_vec();
        let link_future = hashweb.store(content.clone());
        let link = reactor.run(link_future).unwrap();
        //assert_eq!( *link.hashspace(), default_space );
        assert!(link.starts_with((default_space + HashWebLink_HashSpaceId_Separator).as_str()));

        let bytes_future = hashweb.resolve(&link);
        let bytes = reactor.run(bytes_future).unwrap();
        assert_eq!(bytes, content);
    }
}
