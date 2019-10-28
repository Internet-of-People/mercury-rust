use std::collections::HashMap;
use std::hash::Hash;

use failure::{err_msg, format_err, Fallible};
use futures::future;
use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::asynch::*;

pub type HashSpaceId = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HashWebLink {
    hashspace: HashSpaceId,
    hash: String,
}

impl HashWebLink {
    pub const HASH_SPACE_ID_SEPARATOR: &'static str = "/";
    pub const ATTRIBUTE_SEPARATOR: &'static str = "#";

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

    pub fn parse(address_str: &str) -> Fallible<HashWebLink> {
        // Ignore starting slash
        let address = if address_str.starts_with('/') { &address_str[1..] } else { address_str };

        // Split hashspaceId and hash parts
        let slash_pos =
            address.find('/').ok_or(format_err!("Failed to parse address {}", address_str))?; //.unwrap_or( address.len() );
        let (hashspace_id, slashed_hash) = address.split_at(slash_pos);
        let hash = &slashed_hash[1..]; // Ignore starting slash

        // Perform link resolution
        let hashlink = HashWebLink::new(&hashspace_id.to_string(), hash);
        Ok(hashlink)
    }
}

pub struct HashWeb<ObjectType> {
    hashspaces: HashMap<HashSpaceId, Box<dyn HashSpace<ObjectType, String>>>,
    default: HashSpaceId,
}

impl<ObjectType: 'static> HashWeb<ObjectType> {
    pub fn new(
        hashspaces: HashMap<HashSpaceId, Box<dyn HashSpace<ObjectType, String>>>,
        default: HashSpaceId,
    ) -> Self {
        HashWeb { hashspaces, default }
    }
}

impl<ObjectType> HashSpace<ObjectType, String> for HashWeb<ObjectType>
where
    ObjectType: 'static,
{
    fn store(&mut self, object: ObjectType) -> AsyncFallible<String> {
        let mut hashspace_res = self
            .hashspaces
            .get_mut(&self.default)
            .ok_or(format_err!("Unsupported hash space: {}", self.default));
        let hashspace = match hashspace_res {
            Ok(ref mut space) => space,
            Err(e) => return Box::new(future::err(e)),
        };
        let default_hashspace_clone = self.default.clone();
        let result = hashspace.store(object).map(move |hash| {
            default_hashspace_clone + HashWebLink::HASH_SPACE_ID_SEPARATOR + &hash
        });
        Box::new(result)
    }

    fn resolve(&self, hashlink_str: &String) -> AsyncFallible<ObjectType> {
        let hashlink = match HashWebLink::parse(hashlink_str) {
            Ok(link) => link,
            Err(e) => return Box::new(future::err(e)),
        };

        let hashspace_res = self
            .hashspaces
            .get(hashlink.hashspace())
            .ok_or(format_err!("Unsupported hash space: {}", hashlink.hashspace()));
        let hashspace = match hashspace_res {
            Ok(space) => space,
            Err(e) => return Box::new(future::err(e)),
        };
        let data = hashspace.resolve(&hashlink.hash().to_owned());
        Box::new(data)
    }

    fn validate(&self, object: &ObjectType, hashlink_str: &String) -> AsyncFallible<bool> {
        let hashlink = match HashWebLink::parse(hashlink_str) {
            Ok(link) => link,
            Err(e) => return Box::new(future::err(e)),
        };

        let hashspace_res = self
            .hashspaces
            .get(hashlink.hashspace())
            .ok_or(format_err!("Unsupported hash space: {}", hashlink.hashspace()));
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
            None => Err(err_msg("invalid key")),
        };
        Box::new(result.into_future())
    }

    fn clear_local(&mut self, key: KeyType) -> StorageResult<()> {
        let result = self.map.remove(&key).map(|_| ()).ok_or(err_msg("invalid key"));
        Box::new(result.into_future())
    }
}
#[cfg(test)]
mod tests {
    use multihash;

    use super::*;
    use crate::common::imp::*;
    use tokio_current_thread as reactor;

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

        let mut reactor = reactor::CurrentThread::new();

        let default_space = "cache".to_owned();
        let mut spaces: HashMap<String, Box<dyn HashSpace<Vec<u8>, String>>> = HashMap::new();
        spaces.insert(default_space.clone(), Box::new(cache_space));
        let mut hashweb = HashWeb::new(spaces, default_space.clone());

        let content = b"There's over a dozen netrunners Netwatch Cops would love to brain burn and Rache Bartmoss is at least two of them".to_vec();
        let link_future = hashweb.store(content.clone());
        let link = reactor.block_on(link_future).unwrap();
        assert!(link.starts_with((default_space + HashWebLink::HASH_SPACE_ID_SEPARATOR).as_str()));

        let bytes_future = hashweb.resolve(&link);
        let bytes = reactor.block_on(bytes_future).unwrap();
        assert_eq!(bytes, content);
    }
}
