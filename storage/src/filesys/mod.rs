use ::async::*;
use ::error::*;
use futures::*;
use futures::sync::oneshot;
use std::path::Path;
use std::error::Error;
use std::fs::create_dir_all;
use tokio_io::io::*;
use tokio_fs::*;
use tokio_fs::file::*;
use tokio_threadpool;

pub mod sync;

pub struct AsyncFileHandler{
    path : String,
    pool : tokio_threadpool::ThreadPool,
}

impl AsyncFileHandler{
    pub fn init(main_directory : String) 
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

    pub fn create_subdir(&self, path : String) 
    -> Result<(), StorageError>{
        match create_dir_all(Path::new(&self.get_path(path))){
            Ok(_)=>Ok(()),
            Err(e)=>Err(StorageError::StringError(e.description().to_owned()))
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
    -> Box< Future< Item = (), Error = StorageError > > {
        let (tx, rx) = oneshot::channel::<Result<(), StorageError>>();   
        self.pool.spawn(
            File::create(self.get_path(file_path))
                .map_err(|_err| StorageError::StringError(String::from("File couldn't be created")))
                .and_then(move |file| {                     
                    write_all(file, content)
                        .map(|_| ()) 
                        .map_err(|_e|StorageError::StringError(String::from("Write to file failed")))
                    
                })
                .then(
                    move |res| {                        
                        tx.send(res);  
                        future::ok(())
                    }
                )
        );
    
        Box::new(
            rx
                .or_else(|e| future::err(StorageError::StringError(e.description().to_owned())))
                .and_then(|res| {
                    match res {
                        Ok(()) => future::ok(()),
                        Err(e) => future::err(e)
                    }
                })
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
                    StorageError::StringError(String::from("File couldn't be opened"))
                )
                .and_then(move |file|

                    read_to_end(file , Vec::new())          
                        .map_err(|_e|{ StorageError::StringError(String::from("Read from file failed"))})
                )
                .and_then(|(_, buffer)|
                    match String::from_utf8(buffer) {
                        Ok(content)=> future::ok(content),                                
                        Err(_e)=> future::err(StorageError::StringError(String::from("Failed to convert to UTF-8")))                                
                    }
                )
                .then( move |res| {
                    tx.send(res);
                    future::ok(())                    
                })              
        );
        Box::new(
            rx                                          
            .or_else(|e| future::err(StorageError::StringError(e.description().to_owned())))
            .and_then(|res| {
                match res {
                    Ok(s) => future::ok(s),
                    Err(e) => future::err(e)
                }
            })                
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

#[test]
fn future_file_key_value() {
    use tokio_core;
    use tokio_core::reactor;


    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : AsyncFileHandler = AsyncFileHandler::init(String::from("./ipfs/banan/")).unwrap();
    let file_path = String::from("alma.json");
    let json = String::from("<Json:json>");
    let set = storage.set(file_path.clone(), json.clone());
    reactor.run(set);
    // reactor.run(storage.set(file_path, String::from("<<profile:almagyar>>")));
    let read = storage.get(file_path);
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

