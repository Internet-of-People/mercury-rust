use super::*;


pub const default_addr : String = "127.0.0.1:7070".into();

pub struct ServerConfig{
    pub event_file : Option<String>,
    pub event_timer : Option<u64>,
    pub event_count : Option<u32>,
}

impl ServerConfig{
    pub fn new_from_args(args: ArgMatches)-> Result<Self, std::io::Error> {
        let file_name = args.value_of("event-file").map(|s| s.into());

        let timer = match args.value_of("event-timer") {
            Some(s) => 
                s.parse::<u64>()
                    .map(|i| Some(i))
                    .map_err(|err| 
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --event-timer")),
            _ => 
                Result::Ok(Option::None)
        }?;

        
                    
        let count = match args.value_of("stop-after"){
            Some(s) => {
                s.parse::<u32>()
                    .map(|i| 
                        Some(i))
                    .map_err(|err| 
                        std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --stop-after"))
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
            event_count: count
        })
    }
}

pub struct ClientConfig{
    pub addr: String,
    pub on_fail: OnFail
}

impl ClientConfig{
    pub fn new_from_args(args: ArgMatches)->Result<Self, std::io::Error> {
        let connect_address = args.value_of("connect").map(|s| s.into()).unwrap_or(default_addr);

        let on_fail = match args.value_of("on-fail") {
            Some(fail) => {
                match fail {
                    "retry" => 
                        OnFail::Retry,
                    "terminate" => 
                        OnFail::Terminate,
                    _ => 
                        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "failed to parse --on-fail value"))                    
                }
            },
            None => {
                OnFail::Terminate
            }
        };

        info!("Connect address: {:?}", connect_address);
        info!("On fail: {:?}",on_fail);

        Ok(Self{
            addr: connect_address.to_string(),
            on_fail: on_fail
        })
    }
}