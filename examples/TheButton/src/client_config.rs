use super::*;
use std::net::SocketAddr;


pub const DEFAULT_ADDR : &str = "127.0.0.1:7070";


pub struct ClientConfig{
    pub private_key: PrivateKey,            // private key of the client
    pub server_id: ProfileId,               // public key of the server
    pub server_address: SocketAddr,         // ip address of the server
    pub callee_profile_id : ProfileId,      // profile id of the server app
    pub on_fail: OnFail
}

impl ClientConfig{
    pub fn new_from_args(args: ArgMatches)->Result<Self, std::io::Error> {
        let private_key_file = args.value_of("client-key-file").unwrap();                  // since the option is required, unwrap() is valid here
        let private_key = PrivateKey(std::fs::read(private_key_file)?);
            
        let server_key_file = args.value_of("server-key-file").unwrap();                  // since the option is required, unwrap() is valid here
        let server_id = ProfileId(std::fs::read(server_key_file)?);
    
        let server_address = match args.value_of("home-address").map(|s| s.into()).unwrap_or(DEFAULT_ADDR).parse() {
            Ok(addr) => addr,
            _err => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --connect value"))
        };

        let on_fail = match args.value_of("on-fail") {
            Some(fail) => {
                match fail {
                    "retry" => 
                        OnFail::Retry,
                    "terminate" => 
                        OnFail::Terminate,
                    _ => 
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --on-fail value"))                    
                }
            },
            None => {
                OnFail::Terminate
            }
        };

        let callee_profile_id = ProfileId(args.value_of("connect").unwrap().as_bytes().to_vec()); // option is required

        info!("Server address: {:?}", server_address);
        info!("On fail: {:?}",on_fail);

        Ok(Self{
            private_key: private_key,
            server_address: server_address,
            server_id: server_id,
            callee_profile_id: callee_profile_id,
            on_fail: on_fail
        })
    }
}