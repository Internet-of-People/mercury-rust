use std::{collections::HashMap, env, fs};

use log::*;
use structopt::StructOpt;



pub fn parse_config<T: StructOpt>(config_path: &str) -> T
{
    let cli_args = env::args().collect::<Vec<_>>();
    let file_args = read_config_file(config_path)
        .unwrap_or( Vec::new() ); // TODO should continue if cfg file was not specified but fail otherwise

    let all_args = cli_args.iter().chain( file_args.iter() );
    // println!("File contents: {:?}", all_args.collect::<Vec<_>>() );

    let matches = T::clap().get_matches_from(all_args);
    T::from_clap(&matches)
}

fn read_config_file(config_path: &str) -> Result< Vec<String>, () >
{
    let file_contents = fs::read_to_string(config_path)
        .map_err( |e| warn!("Error reading config file {}: {}", config_path, e) )?;

    let file_keyvals : HashMap<String, String> = toml::from_str(&file_contents)
        .map_err( |e| {
            error!("Error parsing file {}: {}", config_path, e);
            warn!("Note that only `key = 'value'` format is supported in the config file");
        } )?;

    // println!("File contents: {:?}", file_args);
    let file_args = file_keyvals.iter()
        .flat_map( |(key,value)| vec![ format!("--{}", key), value.to_owned() ] )
        .collect();
    Ok(file_args)
}
