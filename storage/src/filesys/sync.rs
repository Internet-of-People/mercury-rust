use ::async::*;
use ::error::*;

use std::path::Path;
use std::fs::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use futures::*;

/* SYNCRONOUS FILESYSTEM HANDLER */

pub struct SyncFileHandler{
    path : String,
    file_map : HashMap<String, String>
}

impl SyncFileHandler{
    pub fn init(name : String) -> Result<Self, StorageError>{
        let mut path = String::new();
        path.push_str(&name);
        match create_dir_all(Path::new(&path)){
            Ok(_)=>Ok(SyncFileHandler{path: name, file_map : HashMap::new()}),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn create_subdir(&self, directory_path : String) -> Result<(), StorageError>{
        match create_dir_all(Path::new(&self.get_path(directory_path))){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn new_file(&self, file_path : String) -> Result<(), StorageError>{
        match File::create(Path::new(&self.get_path(file_path))){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }
    
    pub fn new_file_with_name(&self, directory_path : String, file_name : String) -> Result<(), StorageError>{
        let mut subpath = self.get_path(directory_path);
        subpath.push_str(&file_name);
        match File::create(Path::new(&subpath)){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }


    pub fn write_to_file(&self, file_path : String, content : String) -> Result<(), StorageError>{
        match File::create(Path::new(&self.get_path(file_path))){
            Ok(mut file)=>{
                match file.write_all(content.as_bytes()){
                    Ok(_) => {
                        file.sync_all();
                        Ok(())
                        },
                    Err(e) => Err(StorageError::Other(Box::new(e)))
                } 
            },
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn read_from_file(&self, file_path : String) -> Result<String, StorageError> {
        if !Path::new(&self.get_path(file_path.clone())).exists(){
            return Err(StorageError::InvalidKey);
        }
        match File::open(Path::new(&self.get_path(file_path))){
            Ok(mut file)=>{
                let mut reader = String::new();
                match file.read_to_string(&mut reader){
                    Ok(_) => Ok(reader),
                    Err(e) => Err(StorageError::Other(Box::new(e)))
                }
            },
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn get_path(&self, file_path: String)->String{
        let mut path = String::from(self.path.clone());
        path.push_str(&file_path);
        path
    }
}

impl KeyValueStore<String, String> for SyncFileHandler{
    fn set(&mut self, key: String, value: String)
    -> Box< Future<Item=(), Error=StorageError> >{
        match self.write_to_file(key, value){
            Ok(_)=>Box::new(future::ok(())),
            Err(e)=>Box::new(future::err(e))
        }    
    }

    fn get(&self, key: String)
        -> Box< Future<Item=String, Error=StorageError> >{
        match self.read_from_file(key){
            Ok(content)=>Box::new(future::ok(content)),
            Err(e)=>Box::new(future::err(e))
        } 
    }
}

#[test]
fn medusa_key_value() {
    use tokio_core::reactor;
    
    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : SyncFileHandler = SyncFileHandler::init(String::from("./ipfs/banan/")).unwrap();
    let json = String::from("<Json:json>");
    reactor.run(storage.set(String::from("alma.json"), json.clone())).unwrap();
    let read = storage.get(String::from("alma.json"));
    let res = reactor.run(read).unwrap();
    assert_eq!(res, json);
}
