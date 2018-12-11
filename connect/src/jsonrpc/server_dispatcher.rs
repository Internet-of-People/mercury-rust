use std::rc::Rc;

//use failure::Fail;
use futures::{prelude::*, future::Either, sync::mpsc};
use tokio_codec::{Decoder, Encoder, Framed};
use jsonrpc_pubsub::PubSubHandler;
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};

use mercury_home_protocol::*;
//use mercury_home_protocol::future::select_first;
use ::error::*;
use ::service::*;
use ::jsonrpc::api;



#[derive(Clone)]
pub struct JsonRpcServer
{
    pubsub_dispatcher: Rc< PubSubHandler<api::Session> >,
//    handle: reactor::Handle,
}



impl JsonRpcServer
{
    pub fn new(service: Rc<ConnectService>, handle: reactor::Handle) -> Self
        { Self{ pubsub_dispatcher: api::create(service, handle) } } //, handle } }


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
        let session = api::Session::new( resp_sink.clone() );

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
