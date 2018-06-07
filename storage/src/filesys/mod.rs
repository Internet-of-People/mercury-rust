use ::async::*;
use ::error::*;
use futures::*;
use futures::sync::oneshot;
use std::path::Path;
use std::error::Error;
use std::fs::create_dir_all;
use tokio_io::io::*;
use tokio_fs::*;
use tokio_threadpool;
use serde_json;
use mercury_home_protocol::{Profile, ProfileId};

pub struct AsyncFileHandler{
    path : String,
    pool : tokio_threadpool::ThreadPool,
}

impl AsyncFileHandler{
    pub fn new(main_directory : String) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path: main_directory, 
                    pool : tokio_threadpool::ThreadPool::new(),
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    // TODO: this call will move away the ThreadPool, which still makes sharing of pools impossible across multiple 
    // entities. Some solution need to be invented to permit sharing Rc<ThreadPool> maybe
    pub fn new_with_pool(main_directory : String, pool: tokio_threadpool::ThreadPool) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path: main_directory, 
                    pool : pool,
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    pub fn new_file(&self, file_path : String) 
    -> Box<Future<Item=File, Error=::std::io::Error>> {
        Box::new(File::create(self.get_path(file_path)))
    }
    
    pub fn new_file_with_name(&self, directory_path : String, file_name : String) 
    -> Box< Future<Item=File, Error=::std::io::Error> > {
        let mut subpath = self.get_path(directory_path);
        subpath.push_str(&file_name);
        Box::new(File::create(subpath))
    }

    pub fn write_to_file(&self, file_path : String, content : String) 
    -> Box< Future< Item = (), Error = StorageError > > {
        let (tx, rx) = oneshot::channel::<Result<(), StorageError>>();   
        self.pool.spawn(
            File::create(self.get_path(file_path))
                // TODO: map the error in a way to preserve the original error too
                .map_err(|_err| StorageError::StringError(String::from("File couldn't be created")))
                .and_then(move |file| {                     
                    write_all(file, content)
                        .map(|_| ()) 
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|_e|StorageError::StringError(String::from("Write to file failed")))
                })
                .then( move |res| tx.send(res))
                .map_err(|_| ())
                .map(|_| ())                        

        );
    
        Box::new(
            rx
                .or_else(|e| future::err(StorageError::StringError(e.description().to_owned())))
                .and_then(|res| res )       // unpacking result
                
        )        
    }

    pub fn read_from_file(&self, file_path : String) 
    -> Box< Future< Item = String, Error = StorageError> > {
        let (tx, rx) = oneshot::channel::<Result<String,StorageError>>();
        if !Path::new(&self.get_path(file_path.clone())).exists(){
            return Box::new(future::err(StorageError::InvalidKey));
        }
        self.pool.spawn(
            File::open(self.get_path(file_path))        
                .map_err(|_e| 
                    // TODO: map the error in a way to preserve the original error too
                    StorageError::StringError(String::from("File couldn't be opened"))
                )
                .and_then(move |file|
                    read_to_end(file , Vec::new())          
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|_e|{ StorageError::StringError(String::from("Read from file failed"))})
                )
                .and_then(|(_, buffer)|
                    match String::from_utf8(buffer) {
                        Ok(content)=> future::ok(content),                                
                        // TODO: map the error in a way to preserve the original error too
                        Err(_e)=> future::err(StorageError::StringError(String::from("Failed to convert to UTF-8")))                                
                    }
                )
                .then( move |res| tx.send(res))
                .map_err(|_| ())
                .map(|_| ())             
        );
        Box::new(
            rx                                  
                .or_else(|e| future::err(StorageError::StringError(e.description().to_owned())))
                .and_then(|res| res)                
        )                
    }

    pub fn get_path(&self, file_path: String)
    -> String {
        let mut path = self.path.clone();
        path.push_str(&file_path);
        path
    }
}

impl KeyValueStore<String, String> for AsyncFileHandler{
    fn set(&mut self, key: String, value: String)
    -> Box< Future<Item=(), Error=StorageError> >{
        Box::new(self.write_to_file(key, value).map_err(|_e|StorageError::OutOfDiskSpace ) )    
    }

    fn get(&self, key: String)
    -> Box< Future<Item=String, Error=StorageError> >{
        Box::new(self.read_from_file(key).map_err(|_e|StorageError::InvalidKey ) )
    }
}

impl KeyValueStore<ProfileId, Profile> for AsyncFileHandler{
    fn set(&mut self, key: ProfileId, value: Profile)
    -> Box< Future<Item=(), Error=StorageError> >{
        let res;
        match serde_json::to_string(&value){
            Ok(str_profile)=>{
                match String::from_utf8(key.0) {
                    Ok(str_key)=> {res = self.write_to_file(str_key, str_profile)},                                
                    Err(e)=> {res = Box::new( future::err( StorageError::StringError( e.description().to_owned() ) ) )}                                
                }
            }
            Err(e)=> {res = Box::new( future::err( StorageError::StringError( e.description().to_owned() ) ) )} 
        }
        Box::new( res )    
    }

    fn get(&self, key: ProfileId)
    -> Box< Future<Item=Profile, Error=StorageError> >{
        let res;
        match String::from_utf8(key.0) {
            Ok(content)=> {res = self.read_from_file(content)
                                    
            },                                
            Err(e)=> {res = Box::new(future::err( StorageError::StringError( e.description().to_owned() ) )) }                                
        }
        Box::new( 
            res
                .map_err(|e| StorageError::StringError( e.description().to_owned()))
                .and_then(|profile|{
                    serde_json::from_str(&profile)
                        .map_err(|e| StorageError::StringError( e.description().to_owned()))
                })
        )
    }
}

#[test]
fn future_file_key_value(){
    use tokio_core::reactor;

    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : AsyncFileHandler = AsyncFileHandler::new(String::from("./ipfs/homeserverid/")).unwrap();
    let file_path = String::from("alma.json");
    let json = String::from("<Json:json>");
    let set = storage.set(file_path.clone(), json.clone());
    reactor.run(set).unwrap();
    // reactor.run(storage.set(file_path, String::from("<<profile:almagyar>>")));
    let read = storage.get(file_path);
    let res = reactor.run(read).unwrap();
    assert_eq!(res, json);
}
