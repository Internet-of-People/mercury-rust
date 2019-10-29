use std::path::{Path, PathBuf};

use failure::Fallible;
use log::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::*;

// NOTE The idea is that choosing between different implementations is possible here. We will see.
//pub type FileStore = AsyncFileStore;
pub type FileStore = BlockingFileStore;

pub struct BlockingFileStore {
    base_path: PathBuf,
}

impl BlockingFileStore {
    pub fn new(base_path_str: &Path) -> Fallible<Self> {
        Ok(Self { base_path: base_path_str.to_owned() })
    }

    fn set_bytes(&self, key: String, value: &[u8]) -> std::io::Result<()> {
        use std::{
            fs::{create_dir_all, File},
            io::Write,
        };
        let file_path = self.base_path.join(key);
        create_dir_all(&self.base_path)?;
        let mut file = File::create(file_path)?;
        file.write_all(value)
    }

    fn get_bytes(&self, key: String) -> Result<Vec<u8>, std::io::Error> {
        use std::{fs::File, io::Read};
        let mut file = File::open(self.base_path.join(key))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

#[async_trait(?Send)]
impl<V> KeyValueStore<String, V> for BlockingFileStore
where
    V: 'static + Serialize + DeserializeOwned + Send,
{
    async fn set(&mut self, key: String, value: V) -> Fallible<()> {
        let bytes = serde_json::to_vec(&value)?;
        self.set_bytes(key, &bytes)?;
        Ok(())
    }

    async fn get(&self, key: String) -> Fallible<V> {
        let bytes = self.get_bytes(key)?;
        let v = serde_json::from_slice(&bytes)?;
        Ok(v)
    }

    async fn clear_local(&mut self, key: String) -> Fallible<()> {
        std::fs::remove_file(self.base_path.join(key))?;
        Ok(())
    }
}

pub struct AsyncFileStore {
    base_path: PathBuf,
    //    runtime: RefCell<tokio::runtime::Runtime>,
}

impl AsyncFileStore {
    pub fn new(base_path_str: &Path) -> Fallible<Self> {
        //        let runtime = tokio::runtime::Runtime::new()?;
        Ok(Self { base_path: base_path_str.to_owned() /*, runtime: RefCell::new(runtime) */ })
    }

    //    fn schedule<T, F>(
    //        &self,
    //        future: F,
    //    ) -> Box<dyn Future<Item = T, Error = failure::Error> + Send + 'static>
    //    where
    //        F: Future<Item = T, Error = failure::Error> + Send + 'static,
    //        T: Send + 'static,
    //    {
    //        trace!("Scheduling file operation on new runtime");
    //        let (tx, rx) = futures::sync::oneshot::channel();
    //        self.runtime
    //            .borrow_mut()
    //            .spawn(future.then(move |r| tx.send(r).map_err(|_| unreachable!())));
    //        let retval = rx.then(|result| {
    //            trace!("File operation finished on new runtime, returning result");
    //            match result {
    //                Ok(val) => val,
    //                Err(e) => {
    //                    trace!("Returning error {}", e);
    //                    Err(err_msg("Channel was cancelled"))
    //                }
    //            }
    //        });
    //        Box::new(retval)
    //    }
}

#[async_trait(?Send)]
impl<V> KeyValueStore<String, V> for AsyncFileStore
where
    V: 'static + Serialize + DeserializeOwned,
{
    async fn set(&mut self, key: String, value: V) -> Fallible<()> {
        let bytes = serde_json::to_vec(&value)?;

        let file_path = self.base_path.join(key);
        trace!("Serialized {} bytes for file contents of {:?}", bytes.len(), file_path.to_str());
        tokio::fs::create_dir_all(self.base_path.clone()).await?;
        trace!("Directory path created");
        let mut file = tokio::fs::File::create(file_path).await?;
        trace!("File opened for write");
        file.write_all(&bytes).await?;
        trace!("File written");
        Ok(())
    }

    async fn get(&self, key: String) -> Fallible<V> {
        trace!("Got file reading request for key {}", key);
        let mut file = tokio::fs::File::open(self.base_path.join(key)).await?;
        trace!("File opened for read");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).await?;
        trace!("Read {} bytes from file", bytes.len());
        let value = serde_json::from_slice(&bytes)?;
        Ok(value)
    }

    async fn clear_local(&mut self, key: String) -> Fallible<()> {
        tokio::fs::remove_file(self.base_path.join(key)).await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use tokio::runtime::current_thread::Runtime;

    #[test]
    fn test_file_store() -> Fallible<()> {
        let mut reactor = Runtime::new()?;
        let mut storage: Box<dyn KeyValueStore<String, String>> =
            Box::new(FileStore::new(&PathBuf::from("./filetest/store/")).unwrap());
        let count = 100;
        let content = "my_application".to_string();
        for i in 0..count {
            let _write = reactor.block_on(storage.set(i.to_string(), content.clone())).unwrap();
            let read = reactor.block_on(storage.get(i.to_string())).unwrap();
            assert_eq!(read, content);
        }
        for i in 0..count {
            let read = reactor.block_on(storage.get(i.to_string())).unwrap();
            let _del = reactor.block_on(storage.clear_local(i.to_string())).unwrap();
            assert_eq!(read, content);
        }
        Ok(())
    }
}
