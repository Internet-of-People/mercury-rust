mod cli;



use crate::cli::*;



fn main() -> Result<(), &'static str>
{
    use structopt::StructOpt;
    let command = Command::from_args();
    println!("{:?}", command);

//    let keyvault = KeyVault{};
//    match command {
//        Command::Create(CreateCommand::Link{my_profile_id, peer_profile_id}) => {
//            let my_profile = match my_profile_id.or( keyvault.get_active() ) {
//                Some(id) => id,
//                None => return Err("Command option my_profile_id is unspecified and no active default profile was found"),
//            };
//            let signer = keyvault.get(&my_profile);
//        },
//        _ => {},
//    };

    Ok( () )
}