use std::collections::HashMap;
use std::hash::Hash;

use crate::sync::*;

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
    KeyType: Eq + Hash + Clone,
    ValueType: Clone,
{
    fn store(&mut self, key: &KeyType, object: ValueType) -> Result<(), StorageError> {
        self.map.insert(key.to_owned(), object);
        Ok(())
    }

    fn lookup(&self, key: &KeyType) -> Result<ValueType, StorageError> {
        self.map.get(&key).map(|v| v.to_owned()).ok_or(StorageError::InvalidKey)
    }
}

#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};

    use super::*;
    use crate::common::imp::*;

    #[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
    struct Person {
        name: String,
        phone: String,
        age: u16,
    }

    #[test]
    fn test_storage() {
        let object =
            Person { name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let hash = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let mut storage: InMemoryStore<Vec<u8>, Person> = InMemoryStore::new();
        let store_res = storage.store(&hash, object.clone());
        assert!(store_res.is_ok());
        let lookup_res = storage.lookup(&hash);
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
    }

    #[test]
    fn test_hashspace() {
        let store: InMemoryStore<Vec<u8>, Vec<u8>> = InMemoryStore::new();
        let mut hashspace: ModularHashSpace<Vec<u8>, Vec<u8>, String> = ModularHashSpace {
            //serializer: Box::new( SerdeJsonSerializer{} ),
            hasher: Box::new(MultiHasher::new(multihash::Hash::Keccak512)),
            storage: Box::new(store),
            hash_coder: Box::new(MultiBaseHashCoder::new(multibase::Base64)),
        };

        //let object = Person{ name: "Aladar".to_string(), phone: "+36202020202".to_string(), age: 28 };
        let object = b"Don't nobody touch nothing".to_vec();
        let store_res = hashspace.store(object.clone());
        assert!(store_res.is_ok());
        let hash = store_res.unwrap();
        let lookup_res = hashspace.resolve(&hash);
        assert!(lookup_res.is_ok());
        assert_eq!(lookup_res.unwrap(), object);
        //        let validate_res = hashspace.validate(&object, &hash);
        //        assert!( validate_res.is_ok() );
        //        assert!( validate_res.unwrap() );
    }
}
