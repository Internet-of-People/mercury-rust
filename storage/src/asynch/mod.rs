use std::rc::Rc;

use failure::{err_msg, format_err, Fallible};
use futures::future;
use futures::prelude::*;

use crate::common::*;

pub mod fs;
pub mod imp;

pub type AsyncResult<T, E> = Box<dyn Future<Item = T, Error = E>>;
pub type AsyncFallible<T> = AsyncResult<T, failure::Error>;

type StorageResult<T> = AsyncFallible<T>;

// TODO probably we should have references (e.g. maybe use AsRef) to keys whenever possible
// NOTE this interface can be potentially implemented using a simple local in-memory storage
//      or something as complex as a distributed hashtable (DHT).
//      If the storage is distributed, removing an entry might not be possible,
//      consider e.g. bittorrent. Consequently we do not provide an operation which removes
//      an entry completely from the whole (distributed) store.
//      Instead, we clear all *local* data and let remaining nodes expire the data if unused.
pub trait KeyValueStore<KeyType, ValueType> {
    fn set(&mut self, key: KeyType, value: ValueType) -> StorageResult<()>;
    fn get(&self, key: KeyType) -> StorageResult<ValueType>;
    fn clear_local(&mut self, key: KeyType) -> StorageResult<()>;
}

use std::marker::PhantomData;
pub struct KeyAdapter<K, V, T: KeyValueStore<K, V>> {
    store: T,
    _k: PhantomData<K>,
    _v: PhantomData<V>,
}

impl<K, V, T: KeyValueStore<K, V>> KeyAdapter<K, V, T> {
    pub fn new(store: T) -> Self {
        Self { store, _k: PhantomData, _v: PhantomData }
    }
}

impl<PreferredKeyType, AvailableKeyType, ValueType, T> KeyValueStore<PreferredKeyType, ValueType>
    for KeyAdapter<AvailableKeyType, ValueType, T>
where
    T: KeyValueStore<AvailableKeyType, ValueType>,
    PreferredKeyType: Into<AvailableKeyType>,
{
    fn set(&mut self, key: PreferredKeyType, value: ValueType) -> StorageResult<()> {
        self.store.set(key.into(), value)
    }

    fn get(&self, key: PreferredKeyType) -> StorageResult<ValueType> {
        self.store.get(key.into())
    }

    fn clear_local(&mut self, key: PreferredKeyType) -> StorageResult<()> {
        self.store.clear_local(key.into())
    }
}

pub trait HashSpace<ObjectType, ReadableHashType> {
    fn store(&mut self, object: ObjectType) -> AsyncFallible<ReadableHashType>;
    fn resolve(&self, hash: &ReadableHashType) -> AsyncFallible<ObjectType>;
    fn validate(&self, object: &ObjectType, hash: &ReadableHashType) -> AsyncFallible<bool>;
}

pub struct ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType> {
    hasher: Rc<dyn Hasher<SerializedType, BinaryHashType>>,
    storage: Box<dyn KeyValueStore<BinaryHashType, SerializedType>>,
    hash_coder: Box<dyn HashCoder<BinaryHashType, ReadableHashType>>,
}

impl<SerializedType, BinaryHashType, ReadableHashType>
    ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
    pub fn new(
        hasher: Rc<dyn Hasher<SerializedType, BinaryHashType>>,
        storage: Box<dyn KeyValueStore<BinaryHashType, SerializedType>>,
        hash_coder: Box<dyn HashCoder<BinaryHashType, ReadableHashType>>,
    ) -> Self {
        Self { hasher, storage, hash_coder }
    }

    fn sync_validate(
        &self,
        serialized_obj: &SerializedType,
        readable_hash: &ReadableHashType,
    ) -> Fallible<bool> {
        let hash_bytes = self.hash_coder.decode(readable_hash)?;
        let valid = self.hasher.validate(&serialized_obj, &hash_bytes)?;
        Ok(valid)
    }
}

impl<SerializedType, BinaryHashType, ReadableHashType> HashSpace<SerializedType, ReadableHashType>
    for ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
where
    SerializedType: 'static,
    BinaryHashType: 'static + Clone,
    ReadableHashType: 'static,
{
    fn store(&mut self, serialized_obj: SerializedType) -> AsyncFallible<ReadableHashType> {
        let hash_bytes_result =
            self.hasher.get_hash(&serialized_obj).map(|obj_hash| (serialized_obj, obj_hash));

        if let Err(e) = hash_bytes_result {
            return Box::new(future::err(e));
        }
        let (serialized_obj, hash_bytes) = hash_bytes_result.unwrap();

        let hash_str_result = self.hash_coder.encode(&hash_bytes);
        if let Err(e) = hash_str_result {
            return Box::new(future::err(e));
        }
        let hash_str = hash_str_result.unwrap();

        let result = self.storage.set(hash_bytes, serialized_obj).map(|_| hash_str);
        Box::new(result)
    }

    fn resolve(&self, hash_str: &ReadableHashType) -> AsyncFallible<SerializedType> {
        let hash_bytes_result = self.hash_coder.decode(&hash_str);
        let hash_bytes = match hash_bytes_result {
            Err(e) => return Box::new(future::err(e)),
            Ok(val) => val,
        };

        let hash_bytes_clone = hash_bytes.clone();
        let hasher_clone = self.hasher.clone();
        let result = self.storage.get(hash_bytes).and_then(move |serialized_obj| {
            match hasher_clone.validate(&serialized_obj, &hash_bytes_clone) {
                Err(e) => Err(e),
                Ok(v) => {
                    if v {
                        Ok(serialized_obj)
                    } else {
                        // TODO consider using a different error code
                        Err(err_msg("Invalid key"))
                    }
                }
            }
        });
        Box::new(result)
    }

    fn validate(
        &self,
        object: &SerializedType,
        hash_str: &ReadableHashType,
    ) -> AsyncFallible<bool> {
        Box::new(future::result(self.sync_validate(&object, &hash_str)))
    }
}
