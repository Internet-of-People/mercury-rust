use std::rc::Rc;
use std::sync::Arc;

//use failure::Fail;
use futures::{prelude::*, future::Either};
use jsonrpc_core::{MetaIoHandler, Params, serde_json as json, types};
use jsonrpc_pubsub::{PubSubHandler, Session, Subscriber, SubscriptionId};
//use tokio_core::reactor;

use mercury_home_protocol::*;
use ::*;
use ::service::*;



pub fn create(service: Rc<ConnectService>) -> Rc<PubSubHandler<Arc<Session>>>
{
    let mut dispatcher = MetaIoHandler::<Arc<Session>>::default();

    dispatcher.add_method("get_session",
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
        move |params: Params|
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

    let mut pubsub = PubSubHandler::<Arc<Session>>::new(dispatcher);
    pubsub.add_subscription( "event",
        ( "subscribe_events", |params: Params, meta: Arc<Session>, subscriber: Subscriber|
        {
            let sink = subscriber.assign_id(SubscriptionId::Number(5)).unwrap();
            std::thread::spawn( move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    match sink.notify(Params::Array(vec![serde_json::Value::Number(10.into())])).wait() {
                        Ok(_) => {},
                        Err(_) => {
                            println!("Subscription has ended, finishing.");
                            break;
                        }
                    }
                }
            } );
        } ),
        ( "unsubscribe_events", |_id: SubscriptionId|
        {
            println!("Closing subscription");
            Ok( serde_json::Value::Bool(true) )
        }  )
    );

    Rc::new(pubsub)
}
