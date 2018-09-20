use std::collections::HashMap;
use std::{env, fs};
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;

use clap;
use toml;

use mercury_home_protocol::{*, crypto::*};



const VERSION: &str = "0.1";


pub struct Config
{
    storage_path: String,
    signer: Rc<Signer>,
    listen_socket: SocketAddr, // TODO consider using Vec if listening on several network devices is needed
}

impl Config
{
    pub fn new<'a>(args: &clap::ArgMatches<'a>) -> Self
    {
        let storage_path = args.value_of(FileCliParser::ARG_NAME_STORAGE_PATH)
            .expect("Storage path should have a default value").to_owned();

        // TODO support hardware wallets
        // NOTE for some test keys see https://github.com/tendermint/signatory/blob/master/src/ed25519/test_vectors.rs
        let private_key_file = args.value_of(FileCliParser::ARG_NAME_SERVER_KEY).expect("failed to open private key file"); 
        // TODO implement base64 and/or multibase parsing
        let private_key = PrivateKey(::std::fs::read(private_key_file).unwrap());
        let signer = Rc::new( Ed25519Signer::new(&private_key)
            .expect("Invalid private key") );

        info!("homenode public key: {}", signer.public_key());
        info!("homenode profile id: {}", signer.profile_id());

        let listen_socket = args.value_of(FileCliParser::ARG_NAME_LOCAL_SOCKET_ADDRESS)
            .expect("Socket address should have a default value")
            .to_socket_addrs().unwrap().next().expect("Failed to parse socket address");

        Self{ storage_path, signer, listen_socket }
    }

    pub fn storage_path(&self) -> &str { &self.storage_path }
    pub fn signer(&self) -> Rc<Signer> { self.signer.clone() }
    pub fn listen_socket(&self) -> &SocketAddr { &self.listen_socket }
}



pub struct FileCliParser {}

impl FileCliParser
{
    pub fn parse_config() -> Config
    {
        let cli_args = env::args().collect::<Vec<_>>();
        let file_args = read_config_file(Self::CONFIG_PATH)
            .unwrap_or( Vec::new() );

        let all_args = cli_args.iter().chain( file_args.iter() );
        // println!("File contents: {:?}", all_args.collect::<Vec<_>>() );

        let matches = Self::config_parser().get_matches_from(all_args);
        Config::new(&matches)
    }


    fn config_parser<'a,'b>() -> clap::App<'a,'b>
    {
        clap::App::new("Mercury Home node")
            .about("Provides an open, distributed, secure communication network")
            .version(VERSION)
// TODO option probably should specify a keyfile instead of the privkey value directly
            .arg( clap::Arg::with_name(Self::ARG_NAME_SERVER_KEY)
                .long(Self::ARG_NAME_SERVER_KEY)
                .case_insensitive(true)
                .takes_value(true)
                .default_value("../etc/homenode.id")
                .value_name("FILE")
                .help("Private key file used to prove server identity. Currently only ed25519 keys are supported in raw binary format")
            )
            .arg( clap::Arg::with_name(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .long(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .overrides_with(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .case_insensitive(true)
                .required(false)
                .takes_value(true)
                .value_name("IP:Port")
                .default_value("0.0.0.0:2077")
                .help("Listen on this socket to serve TCP clients")
            )
            .arg( clap::Arg::with_name(Self::ARG_NAME_STORAGE_PATH)
                .long(Self::ARG_NAME_STORAGE_PATH)
                .overrides_with(Self::ARG_NAME_STORAGE_PATH)
                .case_insensitive(true)
                .required(false)
                .takes_value(true)
                .value_name("path/to/dir")
                .default_value("/tmp/mercury/storage") // TODO only for testing, make this platform-dependent
                .help("Directory path to store persistent data in")
            )
    }


    const CONFIG_PATH: &'static str = "home.cfg";

    const ARG_NAME_SERVER_KEY: &'static str = "server-key";
    const ARG_NAME_LOCAL_SOCKET_ADDRESS: &'static str = "tcp";
    const ARG_NAME_STORAGE_PATH: &'static str = "storage";
}



fn read_config_file(config_path: &str) -> Option< Vec<String> >
{
    let file_contents = fs::read_to_string(config_path)
        .map_err( |e| error!("Error reading file {}: {}", config_path, e) )
        .ok()?;

    let file_keyvals : HashMap<String, String> = toml::from_str(&file_contents)
        .map_err( |e| {
            error!("Error parsing file {}: {}", config_path, e);
            warn!("Note that only `key = 'value'` format is supported in the config file");
        } )
        .ok()?;

    // println!("File contents: {:?}", file_args);
    let file_args = file_keyvals.iter()
        .flat_map( |(key,value)| vec![ format!("--{}", key), value.to_owned() ] )
        .collect();
    Some(file_args)
}



//#[cfg(test)]
//#[test]
//fn test_config_parser()
//{
//    let config = parse_config();
//    println!( "Profile Id: {:?}", config.args.value_of("profile_id") );
//}
