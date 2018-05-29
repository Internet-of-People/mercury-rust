use ::async::*;
use ::error::*;

use futures::*;
use futures::sync::oneshot;
use std::path::Path;
use std::fs::create_dir_all;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::prelude::{Read, Write};
use tokio_fs::*;
use tokio_fs::file::*;
use tokio_threadpool;

pub mod sync;

pub struct FutureFile{
    path : String,
    pool : tokio_threadpool::ThreadPool,
}

impl FutureFile{
    pub fn init(main_directory : String) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                FutureFile{
                    path: main_directory, 
                    pool : tokio_threadpool::ThreadPool::new(),
                }
            ),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn create_subdir(&self, path : String) 
    -> Result<(), StorageError>{
        match create_dir_all(Path::new(&self.get_path(path))){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn new_file(&self, file_path : String) 
    -> CreateFuture<String>{
        File::create(self.get_path(file_path))
    }
    
    pub fn new_file_with_name(&self, directory_path : String, file_name : String) 
    -> CreateFuture<String>{
        let mut subpath = self.get_path(directory_path);
        subpath.push_str(&file_name);
        File::create(subpath)
    }

    pub fn write_to_file(&self, file_path : String, content : String) 
    -> Box< Future< Item = (), Error = StorageError > >{
        let (tx, rx) = oneshot::channel();
        self.pool.spawn({        
            self.new_file(file_path)
                .map_err(|_|())
                .and_then(move |mut file|{
                    //TODO x.write_all gives back Result, is it blocking?
                    file.write_all(content.as_bytes())
                        .map_err(|_|())
                        .and_then(|written|tx.send(written))
                            .map_err(|_|())
                })
        });
        Box::new(
            rx.map_err(|e|StorageError::Other(Box::new(e) ) )
        )
    }

    pub fn read_from_file(&self, file_path : String) 
    -> Box< Future< Item = String, Error = StorageError> > {
        let (tx, rx) = oneshot::channel();
        if !Path::new(&self.get_path(file_path.clone())).exists(){
            return Box::new(future::err(StorageError::InvalidKey));
        }
        self.pool.spawn({        
            File::open(self.get_path(file_path))
                .map_err(|_|())
                .and_then(|mut file|{
                    let mut buffer = String::new();
                    //TODO x.read_to_string gives back Result, is it blocking?
                    file.read_to_string(&mut buffer)
                        .map_err(|_|())
                        .and_then(|_|{
                            tx.send(buffer)
                                .map_err(|_|())
                        })
                })
        });
        Box::new(
            rx.map_err(|e|StorageError::Other(Box::new(e) ) )
        )
    }

    pub fn get_path(&self, file_path: String)
    -> String {
        let mut path = String::from(self.path.clone());
        path.push_str(&file_path);
        path
    }
}

impl KeyValueStore<String, String> for FutureFile{
    fn set(&mut self, key: String, value: String)
    -> Box< Future<Item=(), Error=StorageError> >{
        self.write_to_file(key, value)   
    }

    fn get(&self, key: String)
    -> Box< Future<Item=String, Error=StorageError> >{
        self.read_from_file(key)
    }
}

#[test]
fn future_file_key_value() {
    use tokio_core;
    use tokio_core::reactor;


    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : FutureFile = FutureFile::init(String::from("./ipfs/banan/")).unwrap();
    let json = String::from("<Json:json>");
    let set = storage.set(String::from("alma.json"), json.clone());
    reactor.run(set);
    // reactor.run(storage.set(String::from("alma.json"), String::from("<<profile:almagyar>>")));
    let read = storage.get(String::from("alma.json"));
    let res = reactor.run(read).unwrap();
    assert_eq!(res, json);
}

//test from the tokio_threadpool crate
// #[test]
// fn multi_threadpool() {
//     use futures::sync::oneshot;

//     let pool1 = ThreadPool::new();
//     let pool2 = ThreadPool::new();

//     let (tx, rx) = oneshot::channel();
//     let (done_tx, done_rx) = mpsc::channel();

//     pool2.spawn({
//         rx.and_then(move |_| {
//             done_tx.send(()).unwrap();
//             Ok(())
//         })
//         .map_err(|e| panic!("err={:?}", e))
//     });

//     pool1.spawn(lazy(move || {
//         tx.send(()).unwrap();
//         Ok(())
//     }));

//     done_rx.recv().unwrap();
// }

