use std::rc::Rc;
use std::sync::Arc;

//use failure::Fail;
use futures::{prelude::*, future::Either};
use jsonrpc_core::{MetaIoHandler, Params, serde_json as json, types};
use jsonrpc_pubsub::{PubSubHandler, Session, Subscriber, SubscriptionId};
use tokio_core::reactor;

use mercury_home_protocol::*;
use ::*;
use ::service::*;



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
        move |params: Params, meta: Arc<Session>|
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
                .and_then( |dapp_endpoint| {
                    let resp = Response{ profile_id: dapp_endpoint.selected_profile().into() }; // TODO
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

            handle.spawn( sink
                .notify( Params::Array( vec![serde_json::Value::Number( 10.into() )] ) )
                .map( |_| () )
                .map_err( |_| () )
            )
        } ),
        ( "unsubscribe_events", |_id: SubscriptionId|
        {
            println!("Closing subscription");
            Ok( serde_json::Value::Bool(true) )
        }  )
    );

    Rc::new(pubsub)
}
