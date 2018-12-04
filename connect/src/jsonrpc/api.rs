use std::rc::Rc;
use std::sync::Arc;

//use failure::Fail;
use futures::{prelude::*, future::Either};
use jsonrpc_core::{Metadata, MetaIoHandler, Params, serde_json as json, types};
use jsonrpc_pubsub::{PubSubHandler, Session as PubSubSession, PubSubMetadata, Subscriber, SubscriptionId};
use tokio_core::reactor;

use mercury_home_protocol::*;
use ::*;
use ::service::*;



#[derive(Clone)]
pub struct Session
{
    pubsub_session: Arc<PubSubSession>,
    dapp_session:   Option<Rc<DAppSession>>
}

impl Session
{
    pub fn new(pubsub_session: Arc<PubSubSession>) -> Self
        { Self{ pubsub_session, dapp_session: None } }

    pub fn dapp_session(&self) -> Option<Rc<DAppSession>> { self.dapp_session.clone() }
    pub fn dapp_session_mut(&mut self) -> &mut Option<Rc<DAppSession>> { &mut self.dapp_session }
}

impl Metadata for Session {}

impl PubSubMetadata for Session {
	fn session(&self) -> Option<Arc<PubSubSession>> {
		Some( self.pubsub_session.clone() )
	}
}



pub fn create(service: Rc<ConnectService>, handle: reactor::Handle)
    -> Rc<PubSubHandler<Arc<Session>>>
{
    let mut dispatcher = MetaIoHandler::<Arc<Session>>::default();

    dispatcher.add_method_with_meta("get_session",
    {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
        struct Request {
            application_id: ApplicationId,
            permissions: Option<DAppPermission>,
        }

        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
        struct Response {
            profile_id: String,
        }

        let service = service.clone();
        move |params: Params, mut meta: Arc<Session>|
        {
            let param_map = match params {
                Params::Map(map) => map,
                Params::None     => return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() ),
                Params::Array(_) => return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() ),
            };

            let req = match serde_json::from_value::<Request>( json::Value::Object(param_map) ) {
                Ok(req) => req,
                Err(e)  => {
                    debug!("Invalid parameter format: {}", e);
                    return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() )
                },
            };

            let resp = service.dapp_session(&req.application_id, req.permissions)
                .map_err( |e| types::Error::new(types::ErrorCode::InternalError) ) // TODO
                .and_then( move |dapp_endpoint| {
                    let resp = Response{ profile_id: dapp_endpoint.selected_profile().into() };
                    match Arc::get_mut(&mut meta) {
                        Some(m) => *m.dapp_session_mut() = Some(dapp_endpoint),
                        None => error!("Implementation error: failed to get mutable reference to save dApp session for JsonRpc"),
                    }

                    serde_json::to_value(resp)
                        .map_err( |e| types::Error::new(types::ErrorCode::InternalError) )
                } );
            Either::B(resp)
        }
    } );

    let service_clone = service.clone();
    let mut pubsub = PubSubHandler::<Arc<Session>>::new(dispatcher);
    pubsub.add_subscription( "event",
        ( "subscribe_events", move |params: Params, meta: Arc<Session>, subscriber: Subscriber|
        {
            let sink = match subscriber.assign_id( SubscriptionId::String( "Uninitialized".to_owned() ) )
            {
                Ok(sink) => sink,
                Err(()) => return warn!("Subscription failed"),
            };

            let dapp_session = match meta.dapp_session() {
                Some(s) => s,
                None => return
            };

            let fwd_events_fut = dapp_session.checkin()
                .map_err( |e| () ) // TODO
                .and_then( |dapp_events| dapp_events
                    .map( |event| match event {
                        DAppEvent::PairingResponse(resp) => Params::Array( vec![serde_json::Value::String( "Pairing response".into() )] ),
                        DAppEvent::Call(call) => Params::Array( vec![serde_json::Value::String( "Call".into() )] ),
                    } )
                    .forward( sink.sink_map_err( |e| () ) ) // TODO
                )
                .map( |_| () );

            handle.spawn(fwd_events_fut)
        } ),
        ( "unsubscribe_events", |_id: SubscriptionId|
        {
            println!("Closing subscription");
            Ok( serde_json::Value::Bool(true) )
        }  )
    );

    Rc::new(pubsub)
}
