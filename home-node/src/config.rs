use std::collections::HashMap;
use std::env;
use std::fs;
use std::rc::Rc;

use clap;
use toml;

use mercury_home_protocol::{*, crypto::*};



const VERSION: &str = "0.1";


pub struct Config
{
    signer: Rc<Signer>,
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

        Self{ signer }
    }

    pub fn signer(&self) -> Rc<Signer> { self.signer.clone() }
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
            .arg( clap::Arg::with_name(Self::ARG_NAME_PRIVATE_KEY)
                .long(Self::ARG_NAME_PRIVATE_KEY)
                .aliases(&Self::ARG_ALIASES_PRIVATE_KEY)
                .required(true)
                .case_insensitive(true)
                .takes_value(true)
                .value_name("PRIVATE_KEY")
                .help("Private key used to prove server identity. Currently only ed25519 keys are supported in base64 encoding. TODO.") // TODO
                .overrides_with(Self::ARG_NAME_PRIVATE_KEY)
            )
    }


    const CONFIG_PATH: &'static str = "home.cfg";

    const ARG_NAME_PRIVATE_KEY: &'static str = "private_key";
    const ARG_ALIASES_PRIVATE_KEY: [&'static str; 5] = ["privatekey", "private-key", "secretkey", "secret_key", "secret-key"];
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
