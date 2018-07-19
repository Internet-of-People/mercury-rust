use super::*;

pub struct ServerConfig{
    pub event_file : String,
    pub event_timer : u64,
    pub event_count : u32,
}

impl ServerConfig{
    pub fn new_from_args(args: ArgMatches)->Self{
        let file_name = match args.value_of("event-file"){
            Some(name) => {
                name.to_string()
            }
            None => {
                "".into()
            }
        };

        let timer = match args.value_of("event-timer"){
            Some(time) => {
                match time.parse::<u64>(){
                    Ok(int)=>{
                        int
                    }
                    Err(_e)=>{
                        0
                    }
                }
            }
            None => {
                0
            }
        };

        let count = match args.value_of("stop-after"){
            Some(times) => {
                match times.parse::<u32>(){
                    Ok(int)=>{
                        int
                    }
                    Err(_e)=>{
                        0
                    }
                }
            }
            None => {
                0
            }
        };
        
        info!("File descriptor: {:?}", file_name);
        info!("Event loop timer: {:?}", timer);
        info!("Event count: {:?}", count);

        Self{
            event_file: file_name,
            event_timer: timer,
            event_count: count
        }
    }
}

pub struct ClientConfig{
    pub addr: String,
    pub on_fail: OnFail
}

impl ClientConfig{
    pub fn new_from_args(args: ArgMatches)->Self{
        let connect_address = match args.value_of("connect"){
            Some(addr)=>{
                addr
            }
            None=>{
                "127.0.0.1:7007".into()
            }
        };
        let on_fail = match args.value_of("on-fail"){
            Some(fail) => {
                match fail{
                    "retry" => {
                        OnFail::RETRY
                    }
                    _ => {
                        OnFail::TERMINATE
                    }
                }
            }
            None => {
                OnFail::TERMINATE
            }
        };

        info!("Connect address: {:?}", connect_address);
        info!("On fail: {:?}",on_fail);

        Self{
            addr: connect_address.to_string(),
            on_fail: on_fail
        }
    }
}