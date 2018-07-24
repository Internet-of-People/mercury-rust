use super::*;

pub struct Client{
    cfg: ClientConfig
}

impl Client{
    pub fn new(cfg: ClientConfig) -> Self{
        Self{
            cfg: cfg,
        }
    }

    pub fn run(&self)->i32{
        match self.cfg.on_fail {
            OnFail::Retry => {
                let mut i : u8 = 1;
                while i<33{
                    if self.connect(){
                        Self::on_event();
                        return EX_OK
                    }
                    else{
                        warn!("Connection failed, will try again in {} seconds", i);
                        std::thread::sleep(std::time::Duration::from_secs(i.into()))
                    }
                    i=i*2;
                }
                warn!("Could not connect after repeated tries, exiting app");
                return EX_TEMPFAIL;
            },
            OnFail::Terminate => {
                if self.connect(){
                    Self::on_event();
                    return EX_OK;
                }
                warn!("Could not connect, exiting app");
                return EX_UNAVAILABLE;
            },
        };
    }

    fn connect(&self)->bool{
        match self.cfg.addr.as_str(){
            "addr" => true,
            _ => false
        }
    }

    fn on_event(){
        panic!("TODO on event");
    }
}

impl Future for Client{
    type Item = i32;
    type Error = std::io::Error;
    fn poll(&mut self) ->
        std::result::Result<futures::Async<<Self as futures::Future>::Item>, <Self as futures::Future>::Error>{
            match self.run(){
                0=>Ok(futures::Async::Ready(0)),
                EX_UNAVAILABLE=> Err(std::io::Error::new(std::io::ErrorKind::NotConnected, "NOTCONNECTED")),
                EX_TEMPFAIL=> Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "TIMEOUT")),
                EX_SOFTWARE=> Err(std::io::Error::new(std::io::ErrorKind::Other, "UNDEFINED ERROR")),
                _=>Ok(futures::Async::NotReady)
            }
            
    }
}