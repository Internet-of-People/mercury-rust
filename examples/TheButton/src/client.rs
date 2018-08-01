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
        unimplemented!();
    }

    fn connect(&self)->bool{
        unimplemented!()
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
            unimplemented!();
            
    }
}