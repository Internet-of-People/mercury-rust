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
            let my_profile_id = match my_profile_id.or( vault.get_active_profile() ) {
                Some(id) => id,
                None => return Err("Command option my_profile_id is unspecified and no active default profile was found"),
            };
            let mut profile = vault.get_profile(&my_profile_id);
            let link = profile.create_link(&peer_profile_id);
            println!("Created link: {:?}", link);
        },
        _ => {},
    };

    Ok( () )
}