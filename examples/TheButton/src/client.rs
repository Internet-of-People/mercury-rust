use super::*;
use mercury_connect::sdk::DAppApi;
use mercury_wire::DappConnect;

pub struct Client{
    appctx : AppContext,
    cfg: ClientConfig,
    mercury: Box<DAppApi>
}

enum ClientState {
    Connecting
}

impl Client{
    pub fn new(cfg: ClientConfig, appctx: AppContext) -> Self{
        unimplemented!();
        /*
        Self{
            appctx: appctx,
            cfg: cfg,
            // mercury: Box::new(DappConnect::new(appctx.priv_key))
        }
        */
    }

}

impl Future for Client{
    type Item = i32;
    type Error = std::io::Error;
    fn poll(&mut self) -> std::result::Result<futures::Async<<Self as futures::Future>::Item>, <Self as futures::Future>::Error>
    {

                
    }
}