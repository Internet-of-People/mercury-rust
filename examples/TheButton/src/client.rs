use futures::prelude::*;

use mercury_home_protocol::*;
use super::*;
use ::init_hack::init_client;



pub struct Client
{
    pub cfg: ClientConfig,
    pub appctx: AppContext,
}

impl Client
{
    pub fn new(cfg: ClientConfig, appctx: AppContext) -> Self
        { Self{appctx, cfg} }
}


impl IntoFuture for Client
{
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;
    type Item = ();
    type Error = Error;

    fn into_future(self) -> Self::Future
    {
        let callee_profile_id = self.cfg.callee_profile_id.clone();

        let fut = self.appctx.service.dapp_session(&self.appctx.app_id, None)
            .and_then( move |dapp_session|
            {
                info!("application initialized, calling {:?}", callee_profile_id);
                dapp_session.call(&callee_profile_id, AppMessageFrame(vec![]))
                    .map_err(|err| { error!("call failed: {:?}", err); err } )
            } )
            .and_then( |call|
            {
                info!("call accepted, waiting for incoming messages");
                call.incoming
                    .for_each( |msg: Result<AppMessageFrame, String>| {
                        msg.map( |frame| info!("Client received server message {:?}", frame) )
                           .map_err( |err| warn!("Client got server error {:?}", err) )
                    } )
                    .map_err( |()| Error::from(ErrorKind::CallFailed) )
            } );

        Box::new( init_client(&self).then( |_res| fut ) )
    }
}
