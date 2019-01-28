mod cli;



use prometheus::vault::ProfileVault;
use crate::cli::*;



fn main() -> Result<(), &'static str>
{
    use structopt::StructOpt;
    let command = Command::from_args();
    println!("{:?}", command);

    let vault = ProfileVault{};
    match command {
        Command::Create(CreateCommand::Link{my_profile_id, peer_profile_id}) => {
            let profile_opt = my_profile_id.or( vault.get_active() )
                .and_then( |profile_id| vault.get(&profile_id) );
            let mut profile = match profile_opt {
                Some(profile) => profile,
                None => return Err("Command option my_profile_id is unspecified and no active default profile was found"),
            };
            let link = profile.create_link(&peer_profile_id);
            println!("Created link: {:?}", link);
        },
        _ => {},
    };

    Ok( () )
}