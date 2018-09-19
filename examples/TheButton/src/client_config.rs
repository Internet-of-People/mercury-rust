use super::*;



pub const DEFAULT_ADDR : &str = "127.0.0.1:7070";


pub struct ClientConfig{
    pub callee_profile_id : ProfileId,      // profile id of the server app
    pub on_fail: OnFail
}

impl ClientConfig{
    pub fn try_from(args: &ArgMatches)->Result<Self, std::io::Error> {
        let on_fail = match args.value_of("on-fail") {
            Some(fail) => {
                match fail {
                    "retry"     => OnFail::Retry,
                    "terminate" => OnFail::Terminate,
                    _ => return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --on-fail value"))
                }
            },
            None => OnFail::Terminate
        };
        info!("On fail: {:?}",on_fail);

        let callee_id_str = args.value_of(cli::CLI_SERVER_PROFILE).unwrap();
        let (_base, callee_id_decoded) = ::multibase::decode(callee_id_str).unwrap();

        Ok( Self{callee_profile_id: ProfileId(callee_id_decoded), on_fail} )
    }
}