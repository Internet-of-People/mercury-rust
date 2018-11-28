use std::rc::Rc;

//use failure::Fail;
use futures::{prelude::*, future::Either, sync::mpsc};
use jsonrpc_core::{IoHandler, MetaIoHandler, Params, serde_json as json, types};
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::reactor;
use tokio_io::{AsyncRead, AsyncWrite};

use mercury_home_protocol::*;
//use mercury_home_protocol::future::select_first;
use ::*;
use ::error::*;
use ::service::*;



fn create_core_dispatcher(service: Rc<ConnectService>) -> Rc<IoHandler>
{
    let mut dispatcher = IoHandler::new();

    dispatcher.add_method("session",
    {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
        struct Request {
            app_id: ApplicationId,
            authorization: Option<DAppPermission>,
        }

        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
        struct Response {
            // TODO
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

            let resp = service.dapp_session(&req.app_id, req.authorization)
                .map_err( |e| types::Error::new(types::ErrorCode::InternalError) ) // TODO
                .and_then( |dapp_endpoint| {
                    let resp = Response{}; // TODO
                    serde_json::to_value(resp)
                        .map_err( |e| types::Error::new(types::ErrorCode::InternalError) )
                } );
            Either::B(resp)
        }
    } );

    Rc::new(dispatcher)
}



#[derive(Clone)]
pub struct JsonRpcServer
{
    core_dispatcher: Rc<IoHandler>,
//    handle: reactor::Handle,
}



impl JsonRpcServer
{
    pub fn new(service: Rc<ConnectService>) -> Self //, handle: reactor::Handle) -> Self
        { Self{ core_dispatcher: create_core_dispatcher(service) } } //, handle } }


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

        let req_disp_fut = self.serve_client_requests(net_stream, resp_sink);

//        let fut = select_first( [req_disp_fut, Box::new(fwd_resp_fut)].iter() );
        let fut = req_disp_fut.select(fwd_resp_fut)
            .map( |((),_pending)| () )
            .map_err( |(done,_pending)| done );

        Box::new( fut.map_err( |e| ErrorKind::ImplementationError.into() ) )

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
                            None => Either::A( Ok( () ).into_future() ),
                            Some(response) => Either::B( sender.send(response)
                                .map( |_sink| () ).map_err( |_e| () ) ),
                        }
                    )
            } );
        Box::new(client_fut)
    }
}
