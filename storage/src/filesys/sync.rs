use ::async::*;
use ::error::*;

use std::path::Path;
use std::fs::*;
use std::collections::HashMap;
use std::io::{Read, Write};
use futures::*;

/* SYNCRONOUS FILESYSTEM HANDLER */

pub struct Medusa{
    path : String,
    file_map : HashMap<String, String>
}

impl Medusa{
    pub fn init(name : String) -> Result<Self, StorageError>{
        let mut path = String::new();
        path.push_str(&name);
        match create_dir_all(Path::new(&path)){
            Ok(_)=>Ok(Medusa{path: name, file_map : HashMap::new()}),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn create_subdir(&self, path : String) -> Result<(), StorageError>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&path);
        match create_dir_all(Path::new(&subpath)){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }

    pub fn new_file(&self, path : String) -> Result<(), StorageError>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&path);
        match File::create(Path::new(&subpath)){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }
    
    pub fn new_file_with_name(&self, file_path : String, file_name : String) -> Result<(), StorageError>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&file_path);
        subpath.push_str(&file_name);
        match File::create(Path::new(&subpath)){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::Other(Box::new(e)))
        }
    }


    pub fn write_to_file(&self, file_path : String, content : String) -> Result<(), StorageError>{
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&file_path);
        match File::create(Path::new(&subpath)){
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

    pub fn read_from_file(&self, path : String) -> Result<String, StorageError> {
        let mut subpath = String::from(self.path.clone());
        subpath.push_str(&path);
        match File::open(Path::new(&subpath)){
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

// impl HashSpace<String, Vec<u8>> for Medusa {
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

impl KeyValueStore<String, String> for Medusa{
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
        match self.write_to_file(key, value){
            Ok(_)=>Box::new(future::ok(())),
            Err(e)=>Box::new(future::err(e))
        }    
    }

    fn get(&self, key: String)
        -> Box< Future<Item=String, Error=StorageError> >{
        let path = self.get_path(key.clone());
        let file_path = Path::new(&path);
        if !file_path.exists(){
            return Box::new(future::err(StorageError::InvalidKey));
        }
        match self.read_from_file(key){
            Ok(content)=>Box::new(future::ok(content)),
            Err(e)=>Box::new(future::err(e))
        } 
    }
}

#[test]
fn medusa_key_value() {
    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : Medusa = Medusa::init(String::from("./ipfs/banan/")).unwrap();
    let json = String::from("<Json:json>");
    reactor.run(storage.set(String::from("alma.json"), json.clone())).unwrap();
    let read = storage.get(String::from("alma.json"));
    let res = reactor.run(read).unwrap();
    assert_eq!(res, json);
}
