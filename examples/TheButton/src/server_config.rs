use crate::*;
use log::*;

pub struct ServerConfig {
    pub event_timer: Option<u64>,
    //    pub event_file : Option<String>,
    //    pub event_count : Option<u32>,
}

impl ServerConfig {
    pub fn try_from(args: &ArgMatches) -> Result<Self, Error> {
        let timer = match args.value_of(cli::CLI_EVENT_TIMER) {
            Some(s) => s.parse::<u64>().map(|i| Some(i)).map_err(|e| {
                error!("failed to parse --event-timer");
                Error::from(e.context(ErrorKind::LookupFailed))
            }),
            None => Result::Ok(Option::None),
        }?;

        //        let file_name = args.value_of("event-file").map(|s| s.into());

        //        let count = match args.value_of(cli::CLI_STOP_AFTER){
        //            Some(s) => {
        //                s.parse::<u32>()
        //                    .map(|i|
        //                        Some(i))
        //                    .map_err(|_err|
        //                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --stop-after: {}"))
        //            },
        //            _ =>
        //                Result::Ok(Option::None)
        //
        //        }?;

        // info!("Event loop timer: {:?}", timer);
        //        info!("File descriptor: {:?}", file_name);
        //        info!("Event count: {:?}", count);

        Ok(Self {
            event_timer: timer,
            //            event_file: file_name,
            //            event_count: count,
        })
    }
}
