use ::async::*;
use ::error::*;
use futures::*;
use futures::sync::oneshot;
use std::rc::Rc;
use std::path::{Path, PathBuf};
use std::error::Error;
use std::fs::create_dir_all;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_io::io::*;
use tokio_fs::*;
use tokio_threadpool::{ThreadPool, Builder};
use serde_json;



// TODO Reevaulate this implementation after a lot of enhancements in tokio-fs 0.1.3,
//      especially doing blocking calls like create_dir_all() from std used at multiple places
pub struct AsyncFileHandler{
    path : PathBuf,
    pool : Rc<ThreadPool>,
}

impl AsyncFileHandler{
    pub fn new(main_directory : &str)
    -> Result<Self, StorageError>{
        let thread_pool = Rc::new(ThreadPool::new());
        Self::new_with_pool(main_directory, thread_pool)
    }

    pub fn new_with_pool(main_directory : &str, pool: Rc<ThreadPool>)
    -> Result<Self, StorageError>{
        match create_dir_all(Path::new(&main_directory)){
            Ok(_)=>Ok(
                AsyncFileHandler{
                    path : PathBuf::from(main_directory), 
                    pool : pool,
                }
            ),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
        }
    }

    pub fn new_with_pool_maxblocking(main_directory : &str, max_blocking_size : usize)
    -> Result<Self, StorageError>{
        let thread_pool = Rc::new(Builder::new()
            .max_blocking(max_blocking_size)
            .build()
        );
        Self::new_with_pool(main_directory, thread_pool)
    }

    fn check_and_create_structure<'a>(&self, path : &'a str) -> Result<&'a str, StorageError>{
        match PathBuf::from(path).parent(){
            Some(parent_path)=>{ create_dir_all(self.get_path(parent_path.to_str().unwrap_or("")))
                .map_err(|e| return StorageError::StringError(e.description().to_owned()))?;
            }
            None=>()
        }
        Ok(path)
    }

    pub fn write_to_file(&self, file_path : &str, content : String)
    -> Box< Future< Item = (), Error = StorageError > > {
        let (tx, rx) = oneshot::channel::<Result<(), StorageError>>();
        match self.check_and_create_structure(file_path){
            Ok(checked_path) => {
                trace!("Scheduling write to file {}", checked_path);
                self.pool.spawn( {
                    trace!("Starting write {} bytes to file {}", content.len(), checked_path);
                    File::create(self.get_path(checked_path))
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|e| StorageError::StringError(e.description().to_owned()))
                        .and_then(move |file| {
                            write_all(file, content)
                                .map(|_| ())
                                // TODO: map the error in a way to preserve the original error too
                                .map_err(|e|StorageError::StringError(e.description().to_owned()))
                        })
                        .then( move |res| {
                            trace!("Write to file result {:?}", res);
                            tx.send(res)
                        } )
                        .map_err(|_| ())
                } );
            }
            Err(e) => {return Box::new(future::err(e));}
        }   
        
        Box::new(
            rx.or_else(|e| {
                    debug!("Async file write failed: {:?}", e);
                    future::err(StorageError::StringError(e.description().to_owned()))
                })
                .and_then(|res| res )       // unpacking result
                
        )        
    }

    pub fn read_from_file(&self, file_path : &str)
    -> Box< Future< Item = String, Error = StorageError> > {
        // debug!("Scheduling async file read {}", file_path);
        let (tx, rx) = oneshot::channel::<Result<String,StorageError>>();
        if !&self.get_path(&file_path).exists(){
            return Box::new(future::err(StorageError::InvalidKey));
        }
        let path_clone = self.get_path(&file_path);
        self.pool.spawn(
            Ok( debug!("Opening file {}", file_path) ).into_future()
                .and_then( move |_| File::open(path_clone) )
                .map_err(|e|
                    // TODO: map the error in a way to preserve the original error too
                    StorageError::StringError(e.description().to_owned())
                )
                .and_then(move |file| {
                    debug!("Successfully opened file");
                    read_to_end(file , Vec::new())          
                        // TODO: map the error in a way to preserve the original error too
                        .map_err(|e|{ StorageError::StringError(e.description().to_owned())})
                } )
                .and_then(|(_, buffer)|
                    match String::from_utf8(buffer) {
                        Ok(content)=> future::ok(content),                                
                        // TODO: map the error in a way to preserve the original error too
                        Err(e)=> future::err(StorageError::StringError(e.description().to_owned()))                                
                    }
                )
                .map_err( |e| { debug!("Async read failed: {:?}", e); e } )
                .then( move |res| tx.send(res))
                .map_err( |e| debug!("Sending read result failed: {:?}", e) )
                .map(|_| ())             
        );
        Box::new(
            rx.or_else(|e| {
                    debug!("Async file read failed: {:?}", e);
                    future::err(StorageError::StringError(e.description().to_owned()))
                })
                .and_then(|res| res)                
        )                
    }

    pub fn get_path(&self, file_path: &str)
    -> Box<Path> {
        let mut path = self.path.clone();
        path.push(file_path);
        path.into_boxed_path()
    }

    pub fn remove_file(&self, _file_path: &str) -> Box< Future<Item=(), Error=StorageError> >
    {
        // TODO implement this if file storage will be used
        unimplemented!()
    }
}



impl<V> KeyValueStore<String, V> for AsyncFileHandler
    where  V: 'static + Serialize + DeserializeOwned
{
    fn set(&mut self, key: String, value: V)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        match serde_json::to_string(&value) {
            Ok(str_value) => self.write_to_file(&key, str_value),
            Err(e) => Box::new( future::err( StorageError::StringError( e.description().to_owned() ) ) ),
        }
    }

    fn get(&self, key: String) -> Box< Future<Item=V, Error=StorageError> >
    {
        trace!("Requested reading key {}", key);
        let get_fut = self.read_from_file(&key)
            .inspect( |content| trace!("Read key content of {} bytes", content.len()) )
            .map_err( |e| StorageError::StringError( e.description().to_owned())  )
            .and_then( |profile| serde_json::from_str(&profile)
                .map_err( |e| StorageError::StringError( e.description().to_owned() ) )
            );
        Box::new(get_fut)
    }

    fn clear_local(&mut self, key: String)
        -> Box< Future<Item=(), Error=StorageError> >
    {
        self.remove_file(&key)
    }
}



#[test]
fn future_file_key_value(){
    use tokio_core::reactor;

    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : AsyncFileHandler = AsyncFileHandler::new("./filetest/homeserverid").unwrap();
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
    let mut alpha_storage : AsyncFileHandler = AsyncFileHandler::new_with_pool("./filetest/alpha/", Rc::clone(&thread_pool)).unwrap();
    let mut beta_storage : AsyncFileHandler = AsyncFileHandler::new_with_pool("./filetest/beta", thread_pool).unwrap();
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
