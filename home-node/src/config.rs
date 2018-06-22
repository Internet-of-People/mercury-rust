use std::collections::HashMap;
use std::env;
use std::fs;

use clap;
use toml;

use mercury_home_protocol::*;



const VERSION: &str = "0.1";


pub struct Config
{
    profile_id: ProfileId,
    public_key: PublicKey,
}

impl Config
{
    pub fn new<'a>(args: &clap::ArgMatches<'a>) -> Self
    {
        let public_key = args.value_of( FileCliParser::ARG_NAME_PUBLIC_KEY)
            .expect("Public key should have been mandatory").as_bytes(); // TODO use some encoding, e.g. base56

        let profile_id = ProfileId( b"TODO".to_vec() );
        Self{ public_key: PublicKey( public_key.to_owned() ),
              profile_id }
    }

    pub fn profile_id(&self) -> &ProfileId { &self.profile_id }
    pub fn public_key(&self) -> &PublicKey { &self.public_key }
}



pub struct FileCliParser {}

impl FileCliParser
{
    pub fn parse_config() -> Config
    {
        let cli_args = env::args().collect::<Vec<_>>();
        let file_args = read_config_file(Self::CONFIG_PATH);

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
            .arg( clap::Arg::with_name(Self::ARG_NAME_PUBLIC_KEY)
                .long(Self::ARG_NAME_PUBLIC_KEY)
                .aliases(&Self::ARG_ALIASES_PUBLIC_KEY)
                .required(true)
                .case_insensitive(true)
                .takes_value(true)
                .value_name("PROFILE_ID")
                .help("TODO.")
            )
    }


    const CONFIG_PATH: &'static str = "home.cfg";

    const ARG_NAME_PUBLIC_KEY: &'static str = "public_key";
    const ARG_ALIASES_PUBLIC_KEY: [&'static str; 4] = ["publickey", "public-key", "pub_key", "pub-key"];
}



fn read_config_file(config_path: &str) -> Vec<String>
{
    match fs::read_to_string(config_path)
    {
        Ok(file_contents) =>
        {
            let file_args : HashMap<String, String> = match toml::from_str(&file_contents)
            {
                Ok(toml) => toml,
                Err(e) => panic!( format!("Error parsing file {}: {}\nNote that only `key = 'value'` format is supported in the config file", config_path, e) )
            };
            // println!("File contents: {:?}", file_args);
            file_args.iter()
                .flat_map( |(key,value)| vec![ format!("--{}", key), value.to_owned() ] )
                .collect()
        },
        Err(e) => Vec::new()
    }
}



//#[cfg(test)]
//#[test]
//fn test_config_parser()
//{
//    let config = parse_config();
//    println!( "Profile Id: {:?}", config.args.value_of("profile_id") );
//}