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
            .and_then(move |dapp|
            {
                info!("application initialized, calling {:?}", callee_profile_id);
                dapp.call(&callee_profile_id, AppMessageFrame(vec![]))
                    .map_err(|err| error!("call failed: {:?}", err) )
                    .and_then(|call: DAppCall|
                    {
                        info!("call accepted, waiting for incoming messages");
                        call.receiver.for_each(|msg: Result<AppMessageFrame, String>| {
                            msg.map( |frame| info!("Client received server message {:?}", frame) )
                                .map_err(|errmsg| warn!("Client got server error {:?}", errmsg) )
                        })
                    })
                    .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "encountered error"))
            } );

        let peer_id = self.cfg.callee_profile_id.clone();
        let client_id = self.appctx.client_id.clone();
        let client_id2 = self.appctx.client_id.clone();
        let fut = ::temporary_init_env(&self.appctx)
            .and_then( move |admin|
                admin.relations(&client_id)
                    .map( move |relations| (admin,relations) )
                    .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable) )
            )
            .and_then( move |(admin,relations)|
            {
                let rel_opt = find_relation_proof(&relations, client_id2.clone(), peer_id.clone(),
                    Some(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN) );
                match rel_opt {
                    // TODO return relation found here and continue with that
                    Some(_rel) => Box::new( Ok(()).into_future() ) as Box<Future<Item=_,Error=_>>,
                    // TODO wait for relation response and continue with the call only afterwards
                    None => Box::new( admin.initiate_relation(&client_id2, &peer_id)
                        .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable) ) )
                }
            } )
            .then( |_| fut );

        Box::new(fut)
    }
}

