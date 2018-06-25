use std::collections::HashMap;
use std::env;
use std::fs;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;

use clap;
use toml;

use mercury_home_protocol::{*, crypto::*};



const VERSION: &str = "0.1";


pub struct Config
{
    signer: Rc<Signer>,
    listen_socket: SocketAddr, // TODO consider using Vec if listening on several network devices is needed
}

impl Config
{
    pub fn new<'a>(args: &clap::ArgMatches<'a>) -> Self
    {
        // TODO support hardware wallets
        // NOTE for some test keys see https://github.com/tendermint/signatory/blob/master/src/ed25519/test_vectors.rs
        let private_key_str = args.value_of( FileCliParser::ARG_NAME_PRIVATE_KEY)
            .expect("Private key should be a mandatory option").as_bytes();
        // TODO implement base64 and/or multibase parsing
        let private_key = PrivateKey( b"\x83\x3F\xE6\x24\x09\x23\x7B\x9D\x62\xEC\x77\x58\x75\x20\x91\x1E\x9A\x75\x9C\xEC\x1D\x19\x75\x5B\x7D\xA9\x01\xB9\x6D\xCA\x3D\x42".to_vec() );
        let signer = Rc::new( Ed25519Signer::new(&private_key)
            .expect("Invalid private key") );

        let listen_socket = args.value_of(FileCliParser::ARG_NAME_LOCAL_SOCKET_ADDRESS)
            .expect("Socket address should have a default value")
            .to_socket_addrs().unwrap().next().expect("Failed to parse socket address");

        Self{ signer, listen_socket }
    }

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
            .arg( clap::Arg::with_name(Self::ARG_NAME_PRIVATE_KEY)
                .long(Self::ARG_NAME_PRIVATE_KEY)
                .aliases(&Self::ARG_ALIASES_PRIVATE_KEY)
                .overrides_with(Self::ARG_NAME_PRIVATE_KEY)
                .case_insensitive(true)
                .required(true)
                .takes_value(true)
                .value_name("ENCODED_PRIVATE_KEY")
                .help("Private key used to prove server identity. Currently only ed25519 keys are supported in base64 encoding. TODO") // TODO
            )
            .arg( clap::Arg::with_name(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .long(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .aliases(&Self::ARG_ALIASES_LOCAL_SOCKET_ADDRESS)
                .overrides_with(Self::ARG_NAME_LOCAL_SOCKET_ADDRESS)
                .case_insensitive(true)
                .required(false)
                .takes_value(true)
                .value_name("IP:Port")
                .default_value("0.0.0.0:2077")
                .help("Listen on this socket to serve TCP clients")
            )
    }


    const CONFIG_PATH: &'static str = "home.cfg";

    const ARG_NAME_PRIVATE_KEY: &'static str = "private_key";
    const ARG_ALIASES_PRIVATE_KEY: [&'static str; 5] = ["privatekey", "private-key", "secretkey", "secret_key", "secret-key"];
    const ARG_NAME_LOCAL_SOCKET_ADDRESS: &'static str = "tcp";
    const ARG_ALIASES_LOCAL_SOCKET_ADDRESS: [&'static str; 6] = ["tcpsocket", "tcp_socket", "tcp-socket", "bindtcp", "bind_tcp", "bind-tcp"];
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
