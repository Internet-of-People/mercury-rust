use super::*;
use mercury_home_protocol::*;
use futures::IntoFuture;
use futures::Stream;



pub struct Client {
    cfg: ClientConfig,
    appctx: AppContext,
}

impl Client{
    pub fn new(cfg: ClientConfig, appctx: AppContext) -> Self
        { Self{appctx, cfg} }
}

impl IntoFuture for Client
{
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future
    {
        let callee_profile_id = self.cfg.callee_profile_id.clone();

        let fut = self.appctx.service.dapp_session(&ApplicationId("buttondapp".into()), None )
            .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "Could not initialize MercuryConnect"))
            .and_then(move |mercury_app|
            {
                info!("application initialized, calling {:?}", callee_profile_id);
                mercury_app.call(&callee_profile_id, AppMessageFrame(vec![]))
                    .map_err(|err| error!("call failed: {:?}", err) )
                    .and_then(|call: DAppCall| {
                        info!("call accepted, waiting for incoming messages");
                        call.receiver
                            .for_each(|msg: Result<AppMessageFrame, String>| {
                                msg.map( |frame| info!("got message {:?}", frame) )
                                    .map_err(|errmsg| warn!("got error {:?}", errmsg) )
                            })
                    })
                    .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "encountered error"))
            } );

        let callee_profile_id = self.cfg.callee_profile_id.clone();
        let client_id = self.appctx.client_id.clone();
        let fut = ::temporary_init_env(&self.appctx)
// TODO query if there is already an appropriate connection for the call available
// TODO if there's none only then initiate one and wait for relation response and continue with the call only afterwards
            .and_then( move |admin| admin.initiate_relation(&client_id, &callee_profile_id)
                .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable)) )
            .then( |_| fut );

        Box::new(fut)
    }
}

