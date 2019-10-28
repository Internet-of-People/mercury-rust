use std::cell::RefCell;
use std::path::{Path, PathBuf};

use failure::{err_msg, Fallible};
use futures::prelude::*;
use log::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json;

use crate::asynch::*;

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

    fn set_bytes(&self, key: String, value: &[u8]) -> Result<(), ::std::io::Error> {
        use std::{
            fs::{create_dir_all, File},
            io::Write,
        };
        let file_path = self.base_path.join(key);
        create_dir_all(&self.base_path)?;
        let mut file = File::create(file_path)?;
        file.write_all(value)?;
        Ok(())
    }

    fn get_bytes(&self, key: String) -> Result<Vec<u8>, std::io::Error> {
        use std::{fs::File, io::Read};
        let mut file = File::open(self.base_path.join(key))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

impl<V> KeyValueStore<String, V> for BlockingFileStore
where
    V: 'static + Serialize + DeserializeOwned + Send,
{
    fn set(&mut self, key: String, value: V) -> StorageResult<()> {
        let bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new(Err(e.into()).into_future()),
        };

        let res = self.set_bytes(key, &bytes).map_err(|e| {
            debug!("Failed to write file: {:?}", e);
            e.into()
        });
        Box::new(res.into_future())
    }

    fn get(&self, key: String) -> StorageResult<V> {
        let bytes = match self.get_bytes(key) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new(Err(e.into()).into_future()),
        };

        let res = serde_json::from_slice(&bytes).map_err(|e| e.into());
        Box::new(res.into_future())
    }

    fn clear_local(&mut self, key: String) -> StorageResult<()> {
        let res = std::fs::remove_file(self.base_path.join(key)).map_err(|e| e.into());
        Box::new(res.into_future())
    }
}

pub struct AsyncFileStore {
    base_path: PathBuf,
    runtime: RefCell<tokio::runtime::Runtime>,
}

impl AsyncFileStore {
    pub fn new(base_path_str: &Path) -> Fallible<Self> {
        let runtime = tokio::runtime::Runtime::new()?;
        Ok(Self { base_path: base_path_str.to_owned(), runtime: RefCell::new(runtime) })
    }

    fn schedule<T, F>(
        &self,
        future: F,
    ) -> Box<dyn Future<Item = T, Error = failure::Error> + Send + 'static>
    where
        F: Future<Item = T, Error = failure::Error> + Send + 'static,
        T: Send + 'static,
    {
        trace!("Scheduling file operation on new runtime");
        let (tx, rx) = futures::sync::oneshot::channel();
        self.runtime
            .borrow_mut()
            .spawn(future.then(move |r| tx.send(r).map_err(|_| unreachable!())));
        let retval = rx.then(|result| {
            trace!("File operation finished on new runtime, returning result");
            match result {
                Ok(val) => val,
                Err(e) => {
                    trace!("Returning error {}", e);
                    Err(err_msg("Channel was cancelled"))
                }
            }
        });
        Box::new(retval)
    }
}

impl<V> KeyValueStore<String, V> for AsyncFileStore
where
    V: 'static + Serialize + DeserializeOwned + Send,
{
    fn set(&mut self, key: String, value: V) -> StorageResult<()> {
        let bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new(Err(format_err!("{}", e)).into_future()),
        };

        let file_path = self.base_path.join(key);
        trace!("Serialized {} bytes for file contents of {:?}", bytes.len(), file_path.to_str());
        let fut = tokio::fs::create_dir_all(self.base_path.clone())
            .inspect(|_| trace!("Directory path created"))
            .and_then(move |()| tokio::fs::File::create(file_path))
            .inspect(|_| trace!("File opened for write"))
            .and_then(move |file| tokio::io::write_all(file, bytes))
            .inspect(|_| trace!("File written"))
            .map(|(_file, _buf)| ())
            .map_err(|e| format_err!("Failed to write file: {:?}", e));
        Box::new(self.schedule(fut))
    }

    fn get(&self, key: String) -> StorageResult<V> {
        trace!("Got file reading request for key {}", key);
        let fut = tokio::fs::File::open(self.base_path.join(key))
            .inspect(|_| trace!("File opened for read"))
            .and_then(|file| tokio::io::read_to_end(file, Vec::new()))
            .inspect(|(_file, bytes)| trace!("Read {} bytes from file", bytes.len()))
            .map_err(|e| format_err!("{}", e))
            .and_then(|(_file, bytes)| {
                serde_json::from_slice(&bytes).map_err(|e| {
                    debug!("Failed to read file: {:?}", e);
                    format_err!("{}", e)
                })
            });
        Box::new(self.schedule(fut))
    }

    fn clear_local(&mut self, key: String) -> StorageResult<()> {
        let fut =
            tokio::fs::remove_file(self.base_path.join(key)).map_err(|e| format_err!("{}", e));
        Box::new(self.schedule(fut))
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use tokio_current_thread as reactor;

    #[test]
    fn test_file_store() {
        let mut reactor = reactor::CurrentThread::new();
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
    }
}
