use ::async::*;
use ::error::*;

use futures::*;
use futures::sync::oneshot;
use std::path::Path;
use std::fs::create_dir_all;
use std::collections::HashMap;
use std::io::{Read, Write};
use tokio_fs::*;
use tokio_fs::file::*;
use tokio_threadpool;
use tokio_core;
use tokio_core::reactor;

pub mod sync;

pub struct FutureFile{
    path : String,
    pool : tokio_threadpool::ThreadPool,
    file_map : HashMap<String, String>
}

impl FutureFile{
    pub fn init(main_directory : String) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                FutureFile{
                    path: main_directory, 
                    pool : tokio_threadpool::ThreadPool::new(),
                    file_map : HashMap::new()
                    }
                ),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn create_subdir(&self, path : String) 
    -> Result<(), StorageError>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&path);
        match create_dir_all(Path::new(&subpath)){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn new_file(&self, path : String) 
    -> CreateFuture<String>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&path);
        File::create(subpath)
    }
    
    pub fn new_file_with_name(&self, file_path : String, file_name : String) 
    -> CreateFuture<String>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&file_path);
        subpath.push_str(&file_name);
        File::create(subpath)
    }


    pub fn write_to_file(&self, file_path : String, content : String) 
    -> Box< Future< Item = (), Error = StorageError > >{
        let (tx, rx) = oneshot::channel();
        self.pool.spawn({        
            let mut subpath = String::from(self.path.clone());
            subpath.push_str(&file_path);
            File::create(subpath)
                .map_err(|_|())
                .and_then(move |mut file|{
                    file.write_all(content.as_bytes())
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
        self.pool.spawn({        
            let mut subpath = String::from(self.path.clone());
            subpath.push_str(&file_path);
            File::open(subpath)
                .map_err(|_|())
                .and_then(|mut file|{
                    let mut reader = String::new();
                    file.read_to_string(&mut reader)
                        .map_err(|_|())
                        .and_then(|_|{
                            tx.send(reader)
                                .map_err(|_|())
                        })
                })
                // .map_err(|e|{ StorageError::Other( Box::new(e) ) })
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

// impl HashSpace<String, Vec<u8>> for FutureFile {
//     fn store(&mut self, object: String)
//         -> Box< Future<Item=Vec<u8>, Error=HashSpaceError> >{

//     }
//     fn resolve(&self, hash: &Vec<u8>)
//         -> Box< Future<Item=String, Error=HashSpaceError> >{

//     }
//     fn validate(&self, object: &String, hash: &Vec<u8>)
//         -> Box< Future<Item=bool, Error=HashSpaceError> >{

//     }
// }

impl KeyValueStore<String, String> for FutureFile{
    fn set(&mut self, key: String, value: String)
        -> Box< Future<Item=(), Error=StorageError> >{
        // let mut path = String::new();
        // path.push_str(&key);
        // let file_path = Path::new(&path);
        // if !file_path.exists(){
        //     if self.new_file(path.clone()).is_err(){
        //         return Box::new(future::err(StorageError::OutOfDiskSpace));
        //     }
        // }
        self.write_to_file(key, value)   
    }

    fn get(&self, key: String)
        -> Box< Future<Item=String, Error=StorageError> >{
        let path = self.get_path(key.clone());
        let file_path = Path::new(&path);
        if !file_path.exists(){
            return Box::new(future::err(StorageError::InvalidKey));
        }
        self.read_from_file(key)
    }
}

#[test]
fn future_file_key_value() {
    let pool = tokio_threadpool::ThreadPool::new();
    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : FutureFile = FutureFile::init(String::from("./ipfs/banan/")).unwrap();
    let json = String::from("<Json:json>");
    let set = storage.set(String::from("alma.json"), json.clone());
    reactor.run(set);
    // let set_res = pool.sender().spawn( set );
    reactor.run(storage.set(String::from("alma.json"), json.clone()));
    let read = storage.get(String::from("alma.json"));
    // let read_res = pool.sender().spawn( read );
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

