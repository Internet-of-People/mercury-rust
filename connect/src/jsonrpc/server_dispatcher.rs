use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

//use failure::Fail;
use futures::{prelude::*, future::Either, sync::{mpsc, oneshot}};
use jsonrpc_core::{Metadata, MetaIoHandler, Params, serde_json as json, types};
use jsonrpc_pubsub::{PubSubHandler, Session as PubSubSession, PubSubMetadata, Subscriber, SubscriptionId};
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};

use mercury_home_protocol::*;
//use mercury_home_protocol::future::select_first;
use crate::*;
use crate::error::*;
use crate::service::*;
use crate::jsonrpc::api;



pub struct SessionData
{
    pubsub_session: Arc<PubSubSession>,
    dapp_session:   Option<Rc<DAppSession>>,
    cancel_events:  Option<oneshot::Sender<()>>,
}

impl SessionData
{
    pub fn new(transport_tx: mpsc::Sender<String>) -> Self
    {
        debug!("Creating JsonRpc session");
        Self{ pubsub_session: Arc::new( PubSubSession::new(transport_tx) ),
            dapp_session: None, cancel_events: None }
    }
}

impl Drop for SessionData {
    fn drop(&mut self) {
        debug!("Dropping JsonRpc session");
    }
}



#[derive(Clone)]
pub struct Session
{
    inner: Rc<RefCell<SessionData>>
}

impl Session
{
    pub fn new(transport_tx: mpsc::Sender<String>) -> Self
        { Self{ inner: Rc::new( RefCell::new( SessionData::new(transport_tx) ) ) } }

    pub fn dapp_session(&self) -> Option<Rc<DAppSession>>
        { self.inner.borrow().dapp_session.clone() }
    pub fn set_dapp_session(&mut self, dapp_session: Rc<DAppSession>)
        { self.inner.borrow_mut().dapp_session.replace(dapp_session); }

    pub fn take_cancel_events(&mut self) -> Option<oneshot::Sender<()>>
        { self.inner.borrow_mut().cancel_events.take() }
    pub fn set_cancel_events(&mut self, cancel_events: oneshot::Sender<()>)
        { self.inner.borrow_mut().cancel_events.replace(cancel_events); }
}

impl Metadata for Session {}

impl PubSubMetadata for Session {
	fn session(&self) -> Option<Arc<PubSubSession>> {
		Some( self.inner.borrow().pubsub_session.clone() )
	}
}



#[derive(Clone)]
pub struct JsonRpcServer
{
    pubsub_dispatcher: Rc< PubSubHandler<Session> >,
//    handle: reactor::Handle,
}



impl JsonRpcServer
{
    pub fn new(service: Rc<ConnectService>, handle: reactor::Handle) -> Self
        { Self{ pubsub_dispatcher: create_dispatcher(service, handle) } }


    pub fn serve_duplex_stream<S,C>(&self, duplex_stream: S, codec: C) -> AsyncResult<(),Error>
        where S: 'static + AsyncRead + AsyncWrite,
              C: 'static + Decoder<Item=String, Error=::std::io::Error>
                         + Encoder<Item=String, Error=::std::io::Error>
    {
        let framed = Framed::new(duplex_stream, codec);
        let (net_sink, net_stream) = framed.split();

        // Note that this request channel is not strictly necessary feature-wise, but helps
        // building backpressure against flooding the server with requests
        let (req_sink, req_stream) = mpsc::channel(CHANNEL_CAPACITY);
        let fwd_req_fut = net_stream
            .map_err( |e| warn!("Failed to read request or notification: {}", e) )
            .forward( req_sink.sink_map_err( |e| warn!("Failed to forward request or notification: {}", e) ) )
            .map( |(_stream,_sink)| () );

        // Create a response sender that can be cloned to be moved into different lambdas
        let (resp_sink, resp_stream) = mpsc::channel(CHANNEL_CAPACITY);
        let fwd_resp_fut = resp_stream
            .forward( net_sink.sink_map_err( |e| warn!("Failed to send response or notification: {}", e) ) )
            .map( |(_stream,_sink)| () );

        let req_disp_fut = self.serve_client_requests(req_stream, resp_sink);

//        let fut = select_first( [req_disp_fut, Box::new(fwd_resp_fut)].iter() );
        let fut = req_disp_fut
            .select(fwd_req_fut)
                .map( |((),_pending)| () )
                .map_err( |(done,_pending)| done )
            .select(fwd_resp_fut)
                .map( |((),_pending)| () )
                .map_err( |(done,_pending)| done );

        Box::new( fut.map_err( |e| ErrorKind::ImplementationError.into() ) )

    }


    // TODO consider error handling
    pub fn serve_client_requests<I>(&self, req_stream: I, resp_sink: mpsc::Sender<String>) -> AsyncResult<(), ()>
        where I: 'static + Stream<Item=String>,
              //O: 'static + Sink<SinkItem=String> + Clone,
    {
        let session = Session::new( resp_sink.clone() );

        let dispatcher = self.pubsub_dispatcher.clone();
        let client_fut = req_stream
            .map_err( |_e| () )
            .for_each( move |line|
            {
                let sender = resp_sink.clone();
                dispatcher.handle_request( &line, session.clone() )
                    .and_then( move |resp_opt|
                        match resp_opt {
                            None => Either::A( Ok( () ).into_future() ),
                            Some(response) => Either::B( sender.send(response)
                                .map( |_sink| () ).map_err( |_e| () ) ),
                        }
                    )
            } );
        Box::new(client_fut)
    }
}



pub fn create_dispatcher(service: Rc<ConnectService>, handle: reactor::Handle)
    -> Rc< PubSubHandler<Session> >
{
    let mut dispatcher = MetaIoHandler::<Session>::default();

    dispatcher.add_method_with_meta("get_session",
    {
        let service = service.clone();
        move |params: Params, mut meta: Session|
        {
            let param_map = match params {
                Params::Map(map) => map,
                Params::None     => return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() ),
                Params::Array(_) => return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() ),
            };

            let req = match serde_json::from_value::<api::GetSessionRequest>( json::Value::Object(param_map) ) {
                Ok(req) => req,
                Err(e)  => {
                    debug!("Invalid parameter format: {}", e);
                    return Either::A( Err( types::Error::new(types::ErrorCode::InvalidParams) ).into_future() )
                },
            };

            let resp = service.dapp_session(&req.application_id, req.permissions)
                .map_err( |e| types::Error::new(types::ErrorCode::InternalError) ) // TODO
                .and_then( move |dapp_endpoint| {
                    let resp = api::GetSessionResponse{ profile_id: dapp_endpoint.selected_profile().into() };
                    meta.set_dapp_session(dapp_endpoint);
                    serde_json::to_value(resp)
                        .map_err( |e| types::Error::new(types::ErrorCode::InternalError) )
                } );
            Either::B(resp)
        }
    } );

    let mut pubsub = PubSubHandler::<Session>::new(dispatcher);
    pubsub.add_subscription( "event",
        ( "subscribe_events", move |params: Params, mut meta: Session, subscriber: Subscriber|
        {
            // TODO set a better ID
            let sink = match subscriber.assign_id( SubscriptionId::String( "TODO_set_a_better_id_here".to_owned() ) )
            {
                Ok(sink) => sink,
                Err(()) => return warn!("Subscription failed"),
            };

            let dapp_session = match meta.dapp_session() {
                Some(s) => s,
                None => return
            };

            let (cancel_tx, cancel_rx) = oneshot::channel();
            meta.set_cancel_events(cancel_tx); // NOTE on a repeated subscribe call this drops the previous tx causing rx to be cancelled

            let fwd_events_fut = dapp_session.checkin()
                .map_err( |e| () ) // TODO
                .and_then( |dapp_events| dapp_events
                    .map( |event| match event {
                        DAppEvent::PairingResponse(resp) => api::EventNotification{ kind: "Pairing response".into() },
                        DAppEvent::Call(call) => api::EventNotification{ kind: "Call".into() },
//                        DAppEvent::PairingResponse(resp) => Params::Array( vec![serde_json::Value::String( "Pairing response".into() )] ),
//                        DAppEvent::Call(call) => Params::Array( vec![serde_json::Value::String( "Call".into() )] ),
                    } )
                    .filter_map( |note| serde_json::to_value(note).ok() ) // TODO log error if there's any
                    .map( |note_json| {
                        let mut map = serde_json::Map::new();
                        map.insert( "data".to_owned(), note_json );
                        Params::Map(map)
                    } )
                    .forward( sink.sink_map_err( |e| () ) ) // TODO
                )
                .map( |_| () );

            let subscribe_fut = fwd_events_fut.select( cancel_rx.map_err( |_e| () ) ) // TODO
                .map( |((),_pending)| () )
                .map_err( |((),_pending)| () ); // TODO

            handle.spawn( subscribe_fut)
        } ),
        ( "unsubscribe_events", |id: SubscriptionId, mut meta: Session|
        {
            // info!("Cancelling subscription");
            meta.take_cancel_events()
                .ok_or( {
                    debug!("Failed to get channel for cancel signal");
                    jsonrpc_core::Error::internal_error()
                } ) // TODO
                .and_then( |cancel| cancel.send( () )
                    .map_err( |_sentitem| {
                        debug!("Failed to send cancel signal");
                        jsonrpc_core::Error::internal_error()
                    } ) )
                .map( |_| serde_json::Value::Bool(true) )
        }  )
    );

    Rc::new(pubsub)
}
