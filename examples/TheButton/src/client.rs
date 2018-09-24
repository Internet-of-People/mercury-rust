use super::*;
use mercury_home_protocol::*;
use futures::IntoFuture;
use futures::Stream;



pub struct Client {
    appcx : AppContext,
    cfg: ClientConfig,
}

impl Client{
    pub fn new(cfg: ClientConfig, appcx: AppContext) -> Self{
        Self{
            appcx: appcx,
            cfg: cfg,
        }
    }
}

impl IntoFuture for Client {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let callee_profile_id = self.cfg.callee_profile_id.clone();

// TODO query if there is already an appropriate connection for the call available
        let fut = self.appcx.service.dapp_session( &ApplicationId("buttondapp".into()), None )
            .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "Could not initialize MercuryConnect"))
            .and_then(move |mercury_app|{
                info!("application initialized, calling {:?}", callee_profile_id);
                mercury_app.call(&callee_profile_id, AppMessageFrame(vec![]))
                    .map_err(|err| error!("call failed: {:?}", err) )
                    .and_then(|call: Call| {
                        info!("call accepted, waiting for incoming messages");
                        call.receiver
                            .for_each(|msg: Result<AppMessageFrame, String>| {
                                msg.map( |frame| info!("got message {:?}", frame) )
                                    .map_err(|errmsg| warn!("got error {:?}", errmsg) )
                            })
                    })
                    .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "encountered error"))
            });

        let callee_profile_id = self.cfg.callee_profile_id.clone();
        let client_id = self.appcx.client_id.clone();
        let fut = ::temporary_init_env(&self.appcx)
            .and_then( move |admin| admin.initiate_relation(&client_id, &callee_profile_id)
                .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable)) )
// TODO wait for relation response and continue with the call only afterwards
            .then( |_| fut );

        Box::new(fut)
    }
}

