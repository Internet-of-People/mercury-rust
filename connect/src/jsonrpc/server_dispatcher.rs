use std::rc::Rc;
use std::sync::Arc;

use jsonrpc_core::{Metadata, MetaIoHandler, Params, Value};
use jsonrpc_core::futures::Future;
use jsonrpc_pubsub::{PubSubHandler, PubSubMetadata, Session, Subscriber, SubscriptionId};
use jsonrpc_tcp_server::{ServerBuilder, RequestContext};

use mercury_home_protocol::*;
use ::{Contact, DAppCall, DAppEndpoint, DAppEvent, DAppPermission, DAppSession, Error};
use ::jsonrpc::DAppSessionParams;



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



pub struct DAppEndpointDispatcherJsonRpc
{
    endpoint: Rc<DAppEndpoint>,
}


impl DAppEndpointDispatcherJsonRpc
{
    pub fn new(endpoint: Rc<DAppEndpoint>) -> Self
        { Self{endpoint} }

    pub fn serve(&self, socket: &str)
    {
        let mut io = PubSubHandler::new( MetaIoHandler::default() );

        let endpoint = self.endpoint.clone();
        io.add_method("session", move |params : Params| {
            let params: DAppSessionParams = params.parse()?;
//            let session = endpoint.dapp_session(&params.dapp, params.authorization);
            Ok( Value::String( params.dapp.0 ) )
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
            .start( &socket.parse().unwrap() )
            .expect("Server must start with no issues.");

        server.wait();
    }
}


impl DAppEndpoint for DAppEndpointDispatcherJsonRpc
{
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >
    {
        unimplemented!()
    }
}


// impl DAppSession
