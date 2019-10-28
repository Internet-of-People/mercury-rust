use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use failure::{err_msg, Fallible};
use futures::prelude::*;

use crate::common::*;

pub mod fs;
pub mod imp;

pub type AsyncResult<'a, T, E> = future::BoxFuture<'a, Result<T, E>>;
pub type AsyncFallible<'a, T> = AsyncResult<'a, T, failure::Error>;

pub type AsyncLocalResult<'a, T, E> = future::LocalBoxFuture<'a, Result<T, E>>;
pub type AsyncLocalFallible<'a, T> = AsyncLocalResult<'a, T, failure::Error>;

pub type AsyncResult01<'a, T, E> =
    Pin<Box<dyn futures01::future::Future<Item = T, Error = E> + Send + 'a>>;
pub type AsyncFallible01<'a, T> = AsyncResult01<'a, T, failure::Error>;

// TODO probably we should have references (e.g. maybe use AsRef) to keys whenever possible
// NOTE this interface can be potentially implemented using a simple local in-memory storage
//      or something as complex as a distributed hashtable (DHT).
//      If the storage is distributed, removing an entry might not be possible,
//      consider e.g. bittorrent. Consequently we do not provide an operation which removes
//      an entry completely from the whole (distributed) store.
//      Instead, we clear all *local* data and let remaining nodes expire the data if unused.
#[async_trait]
pub trait KeyValueStore<KeyType, ValueType> {
    async fn set(&mut self, key: KeyType, value: ValueType) -> Fallible<()>;
    async fn get(&self, key: KeyType) -> Fallible<ValueType>;
    async fn clear_local(&mut self, key: KeyType) -> Fallible<()>;
}

pub trait KeyValueStore01<KeyType, ValueType> {
    fn set(&mut self, key: KeyType, value: ValueType) -> AsyncFallible01<'_, ()>;
    fn get(&self, key: KeyType) -> AsyncFallible01<'_, ValueType>;
    fn clear_local(&mut self, key: KeyType) -> AsyncFallible01<'_, ()>;
}

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

#[async_trait]
impl<
        PreferredKeyType: Send + Sync + 'static,
        AvailableKeyType: Send + Sync,
        ValueType: Send + Sync,
        T: Send + Sync,
    > KeyValueStore<PreferredKeyType, ValueType> for KeyAdapter<AvailableKeyType, ValueType, T>
where
    T: KeyValueStore<AvailableKeyType, ValueType>,
    PreferredKeyType: Into<AvailableKeyType>,
{
    async fn set(&mut self, key: PreferredKeyType, value: ValueType) -> Fallible<()> {
        self.store.set(key.into(), value).await
    }

    async fn get(&self, key: PreferredKeyType) -> Fallible<ValueType> {
        self.store.get(key.into()).await
    }

    async fn clear_local(&mut self, key: PreferredKeyType) -> Fallible<()> {
        self.store.clear_local(key.into()).await
    }
}

impl<K: 'static, V: 'static, T: KeyValueStore<K, V>> KeyValueStore01<K, V> for T {
    fn set(&mut self, key: K, value: V) -> AsyncFallible01<'_, ()> {
        Box::pin(KeyValueStore::set(self, key, value).compat())
    }

    fn get(&self, key: K) -> AsyncFallible01<'_, V> {
        Box::pin(KeyValueStore::get(self, key).compat())
    }

    fn clear_local(&mut self, key: K) -> AsyncFallible01<'_, ()> {
        Box::pin(KeyValueStore::clear_local(self, key).compat())
    }
}

#[async_trait]
pub trait HashSpace<ObjectType, ReadableHashType> {
    async fn store(&mut self, object: ObjectType) -> Fallible<ReadableHashType>;
    async fn resolve(&self, hash: &ReadableHashType) -> Fallible<ObjectType>;
    async fn validate(&self, object: &ObjectType, hash: &ReadableHashType) -> Fallible<bool>;
}

pub struct ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType> {
    hasher: Arc<dyn Hasher<SerializedType, BinaryHashType> + Send + Sync>,
    storage: Box<dyn KeyValueStore<BinaryHashType, SerializedType> + Send + Sync>,
    hash_coder: Box<dyn HashCoder<BinaryHashType, ReadableHashType> + Send + Sync>,
}

impl<
        SerializedType: 'static + Send + Sync,
        BinaryHashType: 'static + Send + Sync + Clone,
        ReadableHashType: 'static + Send + Sync,
    > ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
    pub fn new(
        hasher: Arc<dyn Hasher<SerializedType, BinaryHashType> + Send + Sync>,
        storage: Box<dyn KeyValueStore<BinaryHashType, SerializedType> + Send + Sync>,
        hash_coder: Box<dyn HashCoder<BinaryHashType, ReadableHashType> + Send + Sync>,
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

#[async_trait]
impl<
        SerializedType: 'static + Send + Sync,
        BinaryHashType: 'static + Send + Sync + Clone,
        ReadableHashType: 'static + Send + Sync,
    > HashSpace<SerializedType, ReadableHashType>
    for ModularHashSpace<SerializedType, BinaryHashType, ReadableHashType>
{
    async fn store(&mut self, serialized_obj: SerializedType) -> Fallible<ReadableHashType> {
        let hash_bytes = self.hasher.get_hash(&serialized_obj)?;
        let hash_str = self.hash_coder.encode(&hash_bytes)?;
        self.storage.set(hash_bytes, serialized_obj).await?;
        Ok(hash_str)
    }

    async fn resolve(&self, hash_str: &ReadableHashType) -> Fallible<SerializedType> {
        let hash_bytes = self.hash_coder.decode(&hash_str)?;
        let serialized_obj = self.storage.get(hash_bytes.clone()).await?;
        if self.hasher.validate(&serialized_obj, &hash_bytes)? {
            Ok(serialized_obj)
        } else {
            // TODO consider using a different error code
            Err(err_msg("Invalid key"))
        }
    }

    async fn validate(
        &self,
        object: &SerializedType,
        hash_str: &ReadableHashType,
    ) -> Fallible<bool> {
        self.sync_validate(&object, &hash_str)
    }
}
