use std::path::PathBuf;
use std::rc::Rc;

use failure::Fail;
use futures::{prelude::*, future::Either, sync::mpsc};
use jsonrpc_core::IoHandler;
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use ::error::*;



#[derive(Clone)]
pub struct StreamingJsonRpc
{
    jsonrpc_dispatch: Rc<IoHandler>,
    handle: reactor::Handle,
}


// TODO this should work not only over UDS socket streams, but over any AsyncRead/Write stream
impl StreamingJsonRpc
{
    pub fn new(jsonrpc_dispatch: Rc<IoHandler>, handle: reactor::Handle) -> Self
        { Self{jsonrpc_dispatch, handle} }


    pub fn dispatch<C>(&self, sock_path: &PathBuf, codec: C) -> AsyncResult<(),Error>
        where C: 'static + Decoder<Item=String, Error=::std::io::Error> +
                   Clone + Encoder<Item=String, Error=::std::io::Error>
    {
        let sock = UnixListener::bind(sock_path, &self.handle).unwrap();

        println!("listening on {:?}", sock_path);

        let this = self.clone();
        let server_fut = sock.incoming().for_each( move |(connection, _peer_addr)|
        {
            let _peer_credentials = connection.peer_cred();
            //let framed = LinesCodec::new().framed(connection);
            let framed = Framed::new( connection, codec.clone() );
            let (net_sink, net_stream) = framed.split();

            let (resp_sink, resp_stream) = mpsc::channel(CHANNEL_CAPACITY);
            let fwd_resp_fut = resp_stream
                .forward( net_sink.sink_map_err( |e| warn!("Failed to send response or notification: {}", e) ) )
                .map( |(_stream,_sink)| () );
            this.handle.spawn( fwd_resp_fut );

            // TODO instead of a double spawn we probably should use a fut1.select(fut2) with a single spawn
            let client_fut = this.serve_client_requests(net_stream, resp_sink);
            this.handle.spawn( client_fut.map_err( |()| warn!("Serving client failed") ) );
            Ok( () )
        } );

        Box::new( server_fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )
    }


    // TODO consider error handling
    pub fn serve_client_requests<I,O>(&self, req_stream: I, resp_sink: O) -> AsyncResult<(), ()>
        where I: 'static + Stream<Item=String>,
              O: 'static + Sink<SinkItem=String> + Clone,
    {
        let this = self.clone();

        let client_fut = req_stream
            .map_err( |_e| () )
            .for_each( move |line|
            {
                let sender = resp_sink.clone();
                this.jsonrpc_dispatch.handle_request(&line)
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
