use super::*;

pub struct ServerConfig{
    pub event_file : Option<String>,
    pub event_timer : Option<u64>,
    pub event_count : Option<u32>,
}

impl ServerConfig{
    pub fn new_from_args(args: ArgMatches)-> Result<Self, std::io::Error> {
        let file_name = args.value_of("event-file").map(|s| s.into());
        let timer = match args.value_of(cli::CLI_EVENT_TIMER) {
            Some(s) => 
                s.parse::<u64>()
                    .map(|i| Some(i))
                    .map_err(|_err| 
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --event-timer")),
            _ => 
                Result::Ok(Option::None)
        }?;
               
        let count = match args.value_of(cli::CLI_STOP_AFTER){
            Some(s) => {
                s.parse::<u32>()
                    .map(|i| 
                        Some(i))
                    .map_err(|_err| 
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --stop-after: {}"))
            },
            _ => 
                Result::Ok(Option::None)
            
        }?;
        
        info!("File descriptor: {:?}", file_name);
        info!("Event loop timer: {:?}", timer);
        info!("Event count: {:?}", count);

        Ok(Self{
            event_file: file_name,
            event_timer: timer,
            event_count: count,
        })
    }
}
