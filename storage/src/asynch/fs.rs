use std::cell::RefCell;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use futures::prelude::*;
use log::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json;

use crate::asynch::*;
use crate::error::StorageError;



//pub type FileStore = AsyncFileStore;
pub type FileStore = BlockingFileStore;



pub struct BlockingFileStore
{
    base_path:  PathBuf,
}


impl BlockingFileStore
{
    pub fn new(base_path_str: &str) -> Result<Self, StorageError>
        { Ok( Self{ base_path: base_path_str.into() } ) }

    fn set_bytes(&self, key: String, value: &[u8]) -> Result<(),::std::io::Error>
    {
        use std::{io::Write, fs::{create_dir_all, File}};
        let file_path = self.base_path.join(key);
        create_dir_all(&self.base_path)?;
        let mut file = File::create(file_path)?;
        file.write_all(value)?;
        Ok( () )
    }

    fn get_bytes(&self, key: String) -> Result<Vec<u8>, std::io::Error>
    {
        use std::{io::Read, fs::File};
        let mut file = File::open( self.base_path.join(key) )?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}


impl<V> KeyValueStore<String, V> for BlockingFileStore
    where  V: 'static + Serialize + DeserializeOwned + Send
{
    fn set(&mut self, key: String, value: V) -> AsyncResult<()>
    {
        let bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new( Err( StorageError::StringError( e.description().to_owned() ) ).into_future() ),
        };

        let res = self.set_bytes(key, &bytes)
            .map_err( |e| { debug!("Failed to write file: {:?}", e); StorageError::StringError( e.description().to_owned() ) } );
        Box::new( res.into_future() )

    }

    fn get(&self, key: String) -> AsyncResult<V>
    {
        let bytes = match self.get_bytes(key) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new( Err( StorageError::StringError( e.description().to_owned() ) ).into_future() ),
        };

        let res = serde_json::from_slice(&bytes)
            .map_err( |e| { StorageError::StringError( e.description().to_owned() ) } );
        Box::new( res.into_future() )
    }

    fn clear_local(&mut self, key: String) -> AsyncResult<()>
    {
        let res = std::fs::remove_file( self.base_path.join(key) )
            .map_err( |e| StorageError::StringError( e.description().to_owned() ) );
        Box::new( res.into_future() )
    }
}



pub struct AsyncFileStore
{
    base_path:  PathBuf,
    runtime:    RefCell<tokio::runtime::Runtime>,
}


impl AsyncFileStore
{
    pub fn new(base_path_str: &str) -> Result<Self, StorageError>
    {
        let runtime = tokio::runtime::Runtime::new()
            .map_err( |e| StorageError::StringError( e.description().to_owned() ) )?;
        Ok( Self { base_path: base_path_str.into(),
                   runtime: RefCell::new(runtime) } )
    }


    fn schedule<T,F>(&self, future: F) -> Box< Future<Item=T, Error=StorageError> + Send + 'static >
        where F: Future<Item=T, Error=StorageError> + Send + 'static,
              T: Send + 'static
    {
        trace!("Scheduling file operation on new runtime");
        let (tx, rx) = futures::sync::oneshot::channel();
        self.runtime.borrow_mut().spawn(future.then(move |r| tx.send(r).map_err(|_| unreachable!())));
        let retval = rx.then( |result| {
            trace!("File operation finished on new runtime, returning result");
            match result {
                Ok(val) => val,
                Err(e)  => { trace!("Returning error {}", e); Err( StorageError::StringError( e.description().to_owned() ) ) },
            }
        } );
        Box::new(retval)
    }
}


impl<V> KeyValueStore<String, V> for AsyncFileStore
    where  V: 'static + Serialize + DeserializeOwned + Send
{
    fn set(&mut self, key: String, value: V) -> AsyncResult<()>
    {
        let bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(e) => return Box::new( Err( StorageError::StringError( e.description().to_owned() ) ).into_future() ),
        };

        let file_path = self.base_path.join(key);
        trace!("Serialized {} bytes for file contents of {:?}", bytes.len(), file_path.to_str());
        let fut = tokio_fs::create_dir_all( self.base_path.clone() )
            .inspect( |_| trace!("Directory path created") )
            .and_then( move |()| tokio_fs::File::create(file_path) )
            .inspect( |_| trace!("File opened for write") )
            .and_then( move |file| tokio_io::io::write_all(file, bytes) )
            .inspect( |_| trace!("File written") )
            .map( |(_file,_buf)| () )
            .map_err( |e| { debug!("Failed to write file: {:?}", e); StorageError::StringError( e.description().to_owned() ) } );
        Box::new( self.schedule(fut) )
    }

    fn get(&self, key: String) -> AsyncResult<V>
    {
        trace!("Got file reading request for key {}", key);
        let fut = tokio_fs::File::open( self.base_path.join(key) )
            .inspect( |_| trace!("File opened for read") )
            .and_then( |file| tokio_io::io::read_to_end( file, Vec::new() ) )
            .inspect( |(_file,bytes)| trace!("Read {} bytes from file", bytes.len()) )
            .map_err( |e| StorageError::StringError( e.description().to_owned())  )
            .and_then( |(_file,bytes)| serde_json::from_slice(&bytes)
                .map_err( |e| { debug!("Failed to read file: {:?}", e); StorageError::StringError( e.description().to_owned() ) } ) );
        Box::new( self.schedule(fut) )
    }

    fn clear_local(&mut self, key: String) -> AsyncResult<()>
    {
        let fut = tokio_fs::remove_file( self.base_path.join(key) )
            .map_err( |e| StorageError::StringError( e.description().to_owned() ) );
        Box::new( self.schedule(fut) )
    }
}



#[test]
fn test_file_store()
{
    let mut reactor = tokio_core::reactor::Core::new().unwrap();
    // let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let mut storage : Box<KeyValueStore<String,String>> =
        Box::new( FileStore::new("./filetest/store/").unwrap() );
    let count = 100;
    let content = "my_application".to_string();
    for i in 0..count {
//        let write = runtime.block_on(storage.set( i.to_string(), content.clone() ) ).unwrap();
//        let read  = runtime.block_on( storage.get( i.to_string() ) ).unwrap();
        let write = reactor.run(storage.set( i.to_string(), content.clone() ) ).unwrap();
        let read  = reactor.run( storage.get( i.to_string() ) ).unwrap();
        assert_eq!(read, content);
    }
    for i in 0..count {
//        let read = runtime.block_on( storage.get( i.to_string() ) ).unwrap();
//        let del  = runtime.block_on( storage.clear_local( i.to_string() ) ).unwrap();
        let read = reactor.run( storage.get( i.to_string() ) ).unwrap();
        let del  = reactor.run( storage.clear_local( i.to_string() ) ).unwrap();
        assert_eq!(read, content);

    }
}
