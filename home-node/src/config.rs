use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::prelude::*;

use clap;
use toml;



pub fn parse_config<'a>() -> Config<'a>
{
    let cli_args = env::args().collect::<Vec<_>>();
    let file_args = read_config_file(CONFIG_PATH);

    let all_args = cli_args.iter().chain( file_args.iter() );
    // println!("File contents: {:?}", all_args.collect::<Vec<_>>() );

    let all_args = cli_args.iter().chain( file_args.iter() );
    let matches = config_parser().get_matches_from(all_args);
    Config{ args: matches }
}


pub struct Config<'a>
{
    pub args: clap::ArgMatches<'a>,
}




const VERSION: &str = "0.1";
const CONFIG_PATH: &str = "home.cfg";


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


fn config_parser<'a,'b>() -> clap::App<'a,'b>
{
    clap::App::new("Mercury Home node")
        .about("Provides an open, distributed communication network")
        .version(VERSION)
        .arg( clap::Arg::with_name("profile_id")
            .long("profile_id")
            .aliases(&["profile_id", "profileid", "profile-id", "home_profile_id", "homeprofileid", "home-profile-id"])
            .case_insensitive(true)
            .takes_value(true)
            .value_name("PROFILE_ID")
            .help("TODO.")
        )
}



//#[cfg(test)]
//#[test]
//fn test_config_parser()
//{
//    let config = parse_config();
//    println!( "Profile Id: {:?}", config.args.value_of("profile_id") );
//}
