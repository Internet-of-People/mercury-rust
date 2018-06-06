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
use serde_json;
use mercury_home_protocol::{Profile, ProfileId};

pub mod sync;

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

    pub fn init_with_pool(main_directory : String, pool: tokio_threadpool::ThreadPool) 
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
                        match tx.send(res){
                            Ok(_)=>future::ok(()),
                            Err(_)=>future::err(())
                        }
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
                    match tx.send(res){
                        Ok(_)=>future::ok(()),
                        Err(_)=>future::err(())
                    }                    
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

impl KeyValueStore<ProfileId, Profile> for AsyncFileHandler{
    fn set(&mut self, key: ProfileId, value: Profile)
    -> Box< Future<Item=(), Error=StorageError> >{
        let mut res;
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
        let mut res;
        match String::from_utf8(key.0) {
            Ok(content)=> {res = self.read_from_file(content)
                                    
            },                                
            Err(e)=> {res = Box::new(future::err( StorageError::StringError( e.description().to_owned() ) )) }                                
        }
        Box::new( 
            res
                .map_err(|e| 
                    StorageError::StringError( e.description().to_owned() )
                )
                .and_then(|profile|{
                    serde_json::from_str(&profile)
                        .map_err(|e| 
                            StorageError::StringError( e.description().to_owned() )
                        )
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


//TODO put into test folder
#[test]
fn profile_serialize_async_key_value_test() {
    use tokio_core;
    use tokio_core::reactor;

    
    let profile = Profile::new(
        &ProfileId("userprofile".into()), 
        &PublicKey("userkey".into()), 
        &vec![]
    );

    let homeprofile = Profile::new_home(
        ProfileId("homeprofile".into()), 
        PublicKey("homekey".into()), 
        String::from("/ip4/127.0.0.1/udp/9876").to_multiaddr().unwrap()
    );

    let mut reactor = reactor::Core::new().unwrap();
    println!("\n\n\n");
    let mut storage : AsyncFileHandler = AsyncFileHandler::new(String::from("./ipfs/homeserverid/")).unwrap();

    let set = storage.set(profile.id.clone(), profile.clone());
    let sethome = storage.set(homeprofile.id.clone(), homeprofile.clone());

    reactor.run(set).unwrap();
    reactor.run(sethome).unwrap();

    let read = storage.get(profile.id.clone());
    let readhome = storage.get(homeprofile.id.clone());

    let res = reactor.run(read).unwrap();
    let reshome = reactor.run(readhome).unwrap();
    assert_eq!(res, profile);
    assert_eq!(reshome, homeprofile);
}