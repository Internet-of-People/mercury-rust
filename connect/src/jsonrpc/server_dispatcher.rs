use std::rc::Rc;

//use failure::Fail;
use futures::{prelude::*, future::Either, sync::mpsc};
use jsonrpc_core::{IoHandler, MetaIoHandler, serde_json::Value};
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};

use mercury_home_protocol::*;
use ::error::*;



fn create_core_dispatcher() -> Rc<IoHandler>
{
    let mut dispatcher = IoHandler::new();

    dispatcher.add_method("session", |params| Ok( Value::String("called".to_owned()) ) );

    Rc::new(dispatcher)
}



#[derive(Clone)]
pub struct JsonRpcServer
{
    core_dispatcher: Rc<IoHandler>,
    handle: reactor::Handle,
}



// TODO this should work not only over UDS socket streams, but over any AsyncRead/Write stream
impl JsonRpcServer
{
    pub fn new(handle: reactor::Handle) -> Self
    {
        Self{ core_dispatcher: create_core_dispatcher(), handle }
    }


    pub fn serve_duplex_stream<S,C>(&self, duplex_stream: S, codec: C) -> AsyncResult<(),Error>
        where S: 'static + AsyncRead + AsyncWrite,
              C: 'static + Decoder<Item=String, Error=::std::io::Error>
                         + Encoder<Item=String, Error=::std::io::Error>
    {
        let framed = Framed::new(duplex_stream, codec);
        let (net_sink, net_stream) = framed.split();

        let (resp_sink, resp_stream) = mpsc::channel(CHANNEL_CAPACITY);
        let fwd_resp_fut = resp_stream
            .forward( net_sink.sink_map_err( |e| warn!("Failed to send response or notification: {}", e) ) )
            .map( |(_stream,_sink)| () );
        self.handle.spawn( fwd_resp_fut );

        let req_disp_fut = self.serve_client_requests(net_stream, resp_sink);
        self.handle.spawn( req_disp_fut );

        // TODO connect futures by f1.select(f2)
        //let fut = req_disp_fut.select(fwd_resp_fut);
        //Box::new( fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )

        Box::new( Ok( () ).into_future() )
    }


    // TODO consider error handling
    pub fn serve_client_requests<I,O>(&self, req_stream: I, resp_sink: O) -> AsyncResult<(), ()>
        where I: 'static + Stream<Item=String>,
              O: 'static + Sink<SinkItem=String> + Clone,
    {
        let dispatcher = self.core_dispatcher.clone();
        let client_fut = req_stream
            .map_err( |_e| () )
            .for_each( move |line|
            {
                let sender = resp_sink.clone();
                dispatcher.handle_request(&line)
                    .and_then( move |resp_opt|
                        match resp_opt {
                            None => Either::B( Ok( () ).into_future() ),
                            Some(response) => Either::A( sender.send(response)
                                .map( |_sink| () ).map_err( |_e| () ) ),
                        }
                    )
            } );
        Box::new(client_fut)
    }
}
