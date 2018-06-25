use ::async::*;
use ::error::*;
use futures::*;
use futures::sync::oneshot;
use std::rc::Rc;
use std::path::Path;
use std::error::Error;
use std::fs::create_dir_all;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_io::io::*;
use tokio_fs::*;
use tokio_threadpool::{ThreadPool, Builder};
use serde_json;


pub struct AsyncFileHandler{
    path : String,
    pool : Rc<ThreadPool>,
}

impl AsyncFileHandler{
    pub fn new(main_directory : String) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path: main_directory, 
                    pool : Rc::new(ThreadPool::new()),
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    // TODO: this call will move away the ThreadPool, which still makes sharing of pools impossible across multiple 
    // entities. Some solution need to be invented to permit sharing Rc<ThreadPool> maybe
    pub fn new_with_pool(main_directory : String, pool: Rc<ThreadPool>) 
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path : main_directory, 
                    pool : pool,
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    pub fn new_with_pool_maxblocking(main_directory : String, max_blocking_size : usize) 
    -> Result<Self, StorageError>{
        let thread_pool = Rc::new(Builder::new()
            .max_blocking(max_blocking_size)
            .build()
        );
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path : main_directory, 
                    pool : thread_pool,
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    fn check_and_create_structure(&self, path : String) -> Result<String, StorageError>{
        let path_clone = path.clone();
        let dir_str = path_clone.rsplitn(2,"/").collect::<Vec<_>>();
        if dir_str.len() > 1{
            create_dir_all(self.get_path(dir_str[1].to_string()))
                .map_err(|e| return StorageError::StringError(e.description().to_owned()))?;
        }
        Ok(path)
    }

    pub fn write_to_file(&self, file_path : String, content : String) 
    -> Box< Future< Item = (), Error = StorageError > > {
        let (tx, rx) = oneshot::channel::<Result<(), StorageError>>();
        let path;
        match self.check_and_create_structure(file_path){
            Ok(checked_path) => {path = checked_path;}
            Err(e) => {return Box::new(future::err(e));}
        }   
        self.pool.spawn(
            File::create(self.get_path(path))
                // TODO: map the error in a way to preserve the original error too
                .map_err(|e| StorageError::StringError(e.description().to_owned()))
                .and_then(move |file| {                
                    write_all(file, content)
                        .map(|_| ()) 
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|e|StorageError::StringError(e.description().to_owned()))
                })
                .then( move |res| tx.send(res))
                .map_err(|_| ())
                .map(|_| ())                        

        );
    
        Box::new(
            rx
                .or_else(|e| {
                    println!("{:?}",e.description().to_owned());
                    future::err(StorageError::StringError(e.description().to_owned()))
                })
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
                .map_err(|e| 
                    // TODO: map the error in a way to preserve the original error too
                    StorageError::StringError(e.description().to_owned())
                )
                .and_then(move |file|
                    read_to_end(file , Vec::new())          
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|e|{ StorageError::StringError(e.description().to_owned())})
                )
                .and_then(|(_, buffer)|
                    match String::from_utf8(buffer) {
                        Ok(content)=> future::ok(content),                                
                        // TODO: map the error in a way to preserve the original error too
                        Err(e)=> future::err(StorageError::StringError(e.description().to_owned()))                                
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

impl<V> KeyValueStore<String, V> for AsyncFileHandler
    where  V: 'static + Serialize + DeserializeOwned{
    fn set(&mut self, key: String, value: V)
    -> Box< Future<Item=(), Error=StorageError> >{
        let res;
        match serde_json::to_string(&value){
            Ok(str_value)=>{
                res = self.write_to_file(key, str_value);                                
            }
            Err(e)=> {res = Box::new( future::err( StorageError::StringError( e.description().to_owned() ) ) )} 
        }
        Box::new( res )    
    }

    fn get(&self, key: String)
    -> Box< Future<Item=V, Error=StorageError> >{
        Box::new( 
            self.read_from_file(key)
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
    let mut storage : AsyncFileHandler = AsyncFileHandler::new(String::from("./filetest/homeserverid/")).unwrap();
    let file_path = String::from("alma.json");
    let json = String::from("<Json:json>");
    let set = storage.set(file_path.clone(), json.clone());
    reactor.run(set).unwrap();
    // reactor.run(storage.set(file_path, String::from("<<profile:almagyar>>")));
    let read = storage.get(file_path);
    let res : String = reactor.run(read).unwrap();
    assert_eq!(res, json);
}

#[test]
fn one_pool_multiple_filehandler(){
    //tokio reactor is only needed to read from a file not to write into a file
    use tokio_core::reactor;

    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let thread_pool = Rc::new(Builder::new()
        .max_blocking(200)
        .pool_size(8)
        .build()
    );
    let mut alpha_storage : AsyncFileHandler = AsyncFileHandler::new_with_pool(String::from("./filetest/alpha/"), Rc::clone(&thread_pool)).unwrap();
    let mut beta_storage : AsyncFileHandler = AsyncFileHandler::new_with_pool(String::from("./filetest/beta/"), thread_pool).unwrap();
    let json = String::from("<Json:json>");
    let file_path = String::from("alma.json");
    for i in 0..100{
        reactor.run(alpha_storage.set(String::from(i.to_string()+"/"+&file_path), json.clone())).unwrap();
        reactor.run(beta_storage.set(String::from(i.to_string()+"/"+&file_path), json.clone())).unwrap();
    }
    let aread : String = reactor.run(alpha_storage.get(String::from("99/alma.json"))).unwrap();
    let bread : String = reactor.run(beta_storage.get(String::from("99/alma.json"))).unwrap();
    assert_eq!(aread, bread);
}
