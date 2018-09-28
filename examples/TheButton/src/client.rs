use futures::IntoFuture;
use futures::Stream;

use mercury_home_protocol::*;
use mercury_connect::profile::EventStream;
use super::*;


pub struct Client {
    cfg: ClientConfig,
    appctx: AppContext,
}

impl Client{
    pub fn new(cfg: ClientConfig, appctx: AppContext) -> Self
        { Self{appctx, cfg} }

    fn wait_for_pairing_response(events: EventStream, my_profile_id: ProfileId)
        -> Box< Future<Item=RelationProof, Error=std::io::Error> >
    {
        let fut = events
            .filter_map( move |event|
            {
                debug!("Profile event listener got event");
                if let ProfileEvent::PairingResponse(proof) = event {
                    debug!("Got pairing response, checking peer id: {:?}", proof);
                    if proof.peer_id(&my_profile_id).is_ok()
                        { return Some(proof) }
                }
                return None
            } )
            .take(1)
            .into_future()
            .map_err( |((),__stream)| {
                debug!("Pairing failed");
                std::io::Error::new(std::io::ErrorKind::Other, "Pairing failed")
            } )
            .and_then( |(proof, _stream)| {
                debug!("Got matching pairing response: {:?}", proof);
                proof.ok_or( {
                    debug!("Profile event stream ended without proper response");
                    std::io::Error::new(std::io::ErrorKind::Other, "Got no pairing response")
                } )
            } );
        Box::new(fut)
    }
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
                        call.incoming.for_each(|msg: Result<AppMessageFrame, String>| {
                            msg.map( |frame| info!("Client received server message {:?}", frame) )
                               .map_err(|errmsg| warn!("Client got server error {:?}", errmsg) )
                        })
                    })
                    .map_err(|()| std::io::Error::new(std::io::ErrorKind::Other, "encountered error"))
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
                    Some(proof) => Box::new( Ok(proof).into_future() ) as Box<Future<Item=_,Error=_>>,
                    None => {
                        let client_id3 = client_id2.clone();
                        let rel_fut = admin.initiate_relation(&client_id2, &peer_id)
                            .and_then( move |()| admin.events(&client_id2) )
                            .map_err( |_e| ::std::io::Error::from(::std::io::ErrorKind::AddrNotAvailable) )
                            .and_then( |events| Self::wait_for_pairing_response(events, client_id3) );
                        Box::new(rel_fut)
                    }
                }
            } )
            .then( |_| fut );

        Box::new(fut)
    }
}
