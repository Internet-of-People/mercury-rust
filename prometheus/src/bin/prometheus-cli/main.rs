//use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use log::*;

use prometheus::types::*;
use prometheus::vault::*;
use crate::cli::*;



mod cli;



fn main() -> Result<(), &'static str>
{
    // TODO fix all these unwraps with proper error handling
    log4rs::init_file( "log4rs.yml", Default::default() ).unwrap();

    use structopt::StructOpt;
    let command = Command::from_args();
    info!("Got command {:?}", command);

    let addr = "127.0.0.1:6161".parse().unwrap();
    let timeout = Duration::from_secs(5);
    info!("Initializing profile vault, connecting to {:?}", addr);
    let vault = DummyProfileVault::new(&addr, timeout).unwrap();

//    let vault = FailingProfileVault{};

    process_command(command, &vault)
}



fn selected_profile(vault: &ProfileVault, my_profile_option: Option<ProfileId>)
                    -> Result<Arc<RwLock<Profile>>, &'static str>
{
    let profile_opt = my_profile_option.or( vault.get_active() )
        .and_then( |profile_id| vault.get(&profile_id) );
    let profile = match profile_opt {
        Some(profile) => profile,
        None => return Err("Command option my_profile_id is unspecified and no active default profile was found"),
    };
    Ok(profile)
}

fn on_profile<F>(vault: &ProfileVault, my_profile: Option<ProfileId>, f: F) -> Result<(), &'static str>
where F: FnOnce(&mut Profile) -> ()
{
    let profile_ptr = selected_profile(vault, my_profile)?;
    match profile_ptr.write() {
        Ok(mut profile) => f(&mut *profile),
        Err(_e) => return Err("Implementation error: failed to get write selected profile"),
    };
    Ok( () )
}


fn process_command(command: Command, vault: &ProfileVault) -> Result<(), &'static str>
{
    match command
    {
        Command::Create(CreateCommand::Link{my_profile_id, peer_profile_id}) => {
            on_profile(vault, my_profile_id, |profile| {
                let link = profile.create_link(&peer_profile_id);
                info!("Created link: {:?}", link);
            } )?;
        },

        Command::Create(CreateCommand::Profile) => {
            let created_profile_ptr = vault.create();
            let created_profile = match created_profile_ptr.read() {
                Ok(profile) => profile,
                Err(_e) => return Err("Implementation error: failed to read created profile"),
            };
            info!( "Created profile with id {}", created_profile.id() );
        },

        Command::Clear(ClearCommand::Attribute{my_profile_id, key}) => {
            on_profile(vault, my_profile_id, |profile| {
                profile.clear_attribute(&key);
                info!("Cleared attribute: {:?}", key);
            } )?;
        },

        Command::List(ListCommand::IncomingLinks{my_profile_id}) => {
            on_profile(vault, my_profile_id, |profile| {
                let followers = profile.followers();
                for follower in followers {
                    info!("  Follower: {:?}", follower);
                }
            } )?;
        },

        Command::List(ListCommand::Profiles) => {
            // TODO
        },

        Command::Remove(RemoveCommand::Link{my_profile_id, link_id}) => {
            on_profile(vault, my_profile_id, |profile| {
                profile.remove_link(&link_id);
                info!("Removed link: {:?}", link_id);
            } )?;
        },

        Command::Set(SetCommand::ActiveProfile{my_profile_id}) => {
            vault.set_active(&my_profile_id);
            info!("Active profile was set to {:?}", my_profile_id);
        },

        Command::Set(SetCommand::Attribute{my_profile_id, key, value}) => {
            on_profile(vault, my_profile_id, |profile| {
                info!("Setting attribute {} to {}", key, value);
                profile.set_attribute(key, value);
            } )?;
        },

        Command::Show(ShowCommand::Profile{profile_id}) => {
            // TODO display profile
            // NOTE must also work with a profile that is not ours
        },

        Command::Status => {
            // TODO what status to display?
        },
    };

    Ok( () )
}