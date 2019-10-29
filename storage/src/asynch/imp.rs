use std::collections::HashMap;
use std::hash::Hash;

use failure::{err_msg, format_err, Fallible};
use serde_derive::{Deserialize, Serialize};

use crate::asynch::*;

pub type HashSpaceId = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HashWebLink {
    hash_space: HashSpaceId,
    hash: String,
}

impl HashWebLink {
    pub const HASH_SPACE_ID_SEPARATOR: &'static str = "/";
    pub const ATTRIBUTE_SEPARATOR: &'static str = "#";

    // TODO solve using &str instead of &String
    pub fn new(hashspace: &HashSpaceId, hash: &str) -> Self {
        Self { hash_space: hashspace.to_owned(), hash: hash.to_owned() }
    }

    pub fn hashspace(&self) -> &HashSpaceId {
        &self.hash_space
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
    hashspaces:
        HashMap<HashSpaceId, Box<dyn HashSpace<ObjectType, String> + Send + Sync + 'static>>,
    default: HashSpaceId,
}

impl<ObjectType: 'static> HashWeb<ObjectType> {
    pub fn new(
        hashspaces: HashMap<
            HashSpaceId,
            Box<dyn HashSpace<ObjectType, String> + Send + Sync + 'static>,
        >,
        default: HashSpaceId,
    ) -> Self {
        HashWeb { hashspaces, default }
    }
}

#[async_trait(?Send)]
impl<ObjectType: Send + Sync + 'static> HashSpace<ObjectType, String> for HashWeb<ObjectType> {
    async fn store(&mut self, object: ObjectType) -> Fallible<String> {
        let hash_space = self
            .hashspaces
            .get_mut(&self.default)
            .ok_or(format_err!("Unsupported hash space: {}", self.default))?;
        let hash = hash_space.store(object).await?;
        Ok(self.default.clone() + HashWebLink::HASH_SPACE_ID_SEPARATOR + &hash)
    }

    async fn resolve(&self, hashlink_str: &String) -> Fallible<ObjectType> {
        let hash_link = HashWebLink::parse(hashlink_str)?;
        let hash_space = self
            .hashspaces
            .get(hash_link.hashspace())
            .ok_or(format_err!("Unsupported hash space: {}", hash_link.hashspace()))?;
        let data = hash_space.resolve(&hash_link.hash().to_owned()).await?;
        Ok(data)
    }

    async fn validate(&self, object: &ObjectType, hash_link_str: &String) -> Fallible<bool> {
        let hash_link = HashWebLink::parse(hash_link_str)?;

        let hash_space = self
            .hashspaces
            .get(hash_link.hashspace())
            .ok_or(format_err!("Unsupported hash space: {}", hash_link.hashspace()))?;
        // TODO to_string() is unnecessary below, find out how to transform signatures so as it's not needed
        let result = hash_space.validate(object, &hash_link.hash().to_owned()).await?;
        Ok(result)
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

#[async_trait(?Send)]
impl<KeyType, ValueType> KeyValueStore<KeyType, ValueType> for InMemoryStore<KeyType, ValueType>
where
    KeyType: Eq + Hash + 'static,
    ValueType: Clone + 'static,
{
    async fn set(&mut self, key: KeyType, object: ValueType) -> Fallible<()> {
        self.map.insert(key, object);
        Ok(())
    }

    async fn get(&self, key: KeyType) -> Fallible<ValueType> {
        match self.map.get(&key) {
            Some(val) => Ok(val.to_owned()),
            None => Err(err_msg("invalid key")),
        }
    }

    async fn clear_local(&mut self, key: KeyType) -> Fallible<()> {
        match self.map.remove(&key) {
            Some(_val) => Ok(()),
            None => Err(err_msg("invalid key")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use multihash;
    use tokio::runtime::current_thread::Runtime;

    use super::*;
    use crate::common::imp::*;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person {
        name: String,
        phone: String,
        age: u16,
    }

    #[test]
    fn test_in_memory_store() {
        let mut reactor = Runtime::new().unwrap();

        let object =
            Person { name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let hash = "key".to_string();
        let mut storage: InMemoryStore<String, Person> = InMemoryStore::new();
        let store_res =
            reactor.block_on(KeyValueStore::set(&mut storage, hash.clone(), object.clone()));
        assert!(store_res.is_ok());
        let lookup_res = reactor.block_on(KeyValueStore::get(&storage, hash));
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
    }

    #[test]
    fn test_modular_hash_space() {
        let mut reactor = Runtime::new().unwrap();

        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
            //            Rc::new( SerdeJsonSerializer{} ),
            Arc::new(MultiHasher::new(multihash::Hash::Keccak512)),
            Box::new(store),
            Box::new(MultiBaseHashCoder::new(multibase::Base64)),
        );

        let object = b"What do you get if you multiply six by nine?".to_vec();
        let store_res = reactor.block_on(hashspace.store(object.clone()));
        assert!(store_res.is_ok());
        let hash = store_res.unwrap();
        let lookup_res = reactor.block_on(hashspace.resolve(&hash));
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
        let validate_res = reactor.block_on(hashspace.validate(&object, &hash));
        assert!(validate_res.is_ok());
        assert!(validate_res.unwrap());
    }

    #[tokio::test]
    async fn test_hashweb() -> Fallible<()> {
        let cache_store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let cache_space: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace::new(
            Arc::new(MultiHasher::new(multihash::Hash::Keccak512)),
            Box::new(cache_store),
            Box::new(MultiBaseHashCoder::new(multibase::Base64)),
        );

        let default_space = "cache".to_owned();
        let mut spaces: HashMap<
            String,
            Box<dyn HashSpace<Vec<u8>, String> + Send + Sync + 'static>,
        > = HashMap::new();
        spaces.insert(default_space.clone(), Box::new(cache_space));
        let mut hashweb = HashWeb::new(spaces, default_space.clone());

        let content = b"There's over a dozen netrunners Netwatch Cops would love to brain burn and Rache Bartmoss is at least two of them".to_vec();
        let link = hashweb.store(content.clone()).await?;
        assert!(link.starts_with((default_space + HashWebLink::HASH_SPACE_ID_SEPARATOR).as_str()));

        let bytes = hashweb.resolve(&link).await?;
        assert_eq!(bytes, content);
        Ok(())
    }
}
