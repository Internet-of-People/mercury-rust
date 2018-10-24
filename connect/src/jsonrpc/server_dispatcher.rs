use std::sync::Arc;

use jsonrpc_core::{Metadata, MetaIoHandler, Params, Value};
use jsonrpc_core::futures::Future;
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, Session, Subscriber, SubscriptionId};
use jsonrpc_tcp_server::{ServerBuilder, RequestContext};

use ::jsonrpc::EchoParams;



#[derive(Clone)]
struct Meta {
	session: Option<Arc<Session>>,
}

impl Default for Meta {
	fn default() -> Self { Self{session: None} }
}

impl Metadata for Meta {}
impl PubSubMetadata for Meta {
	fn session(&self) -> Option<Arc<Session>>
		{ self.session.clone() }
}



pub struct DAppSessionDispatcherJsonRpc {}

impl DAppSessionDispatcherJsonRpc
{
    pub fn serve()
    {
        let mut io = PubSubHandler::new( MetaIoHandler::default() );
        io.add_method("echo", |params : Params| {
            let echo_params: EchoParams = params.parse()?;
            Ok( Value::String(echo_params.message) )
        });

        io.add_subscription("notification_message",
            ("subscribe_notification", |_params: Params, _pubsub_metadata, subscriber: Subscriber|
            {
    //            if params != jsonrpc_core::Params::None {
    //				subscriber.reject( jsonrpc_core::Error {
    //					code: jsonrpc_core::ErrorCode::ParseError,
    //					message: "Invalid parameters. Subscription rejected.".into(),
    //					data: None,
    //				}).unwrap();
    //				return;
    //            }

                let sink = subscriber.assign_id(SubscriptionId::Number(5)).unwrap();
                ::std::thread::spawn(move || {
                    loop {
                        ::std::thread::sleep(::std::time::Duration::from_secs(5));
                        match sink.notify(Params::Array(vec![Value::Number(10.into())])).wait() {
                            Ok(_) => {},
                            Err(_) => {
                                println!("Subscription has ended, finishing.");
                                break;
                            }
                        }
                    }
                });
            } ),
            ("unsubscribe_notification", |_subscriber_id|
            {
                Ok( Value::Bool(true) )
            } ) );

        let server = ServerBuilder::new(io)
            .session_meta_extractor(|context: &RequestContext|
                Meta { session: Some(Arc::new(Session::new(context.sender.clone()))), } )
            .start( &"0.0.0.0:2222".parse().unwrap() )
            .expect("Server must start with no issues.");

        server.wait();
    }
}
