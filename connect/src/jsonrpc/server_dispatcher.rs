use std::path::PathBuf;
use std::rc::Rc;

use failure::Fail;
use futures::{prelude::*, future::{loop_fn, Loop}, sync::mpsc};
use jsonrpc_core::IoHandler;
use jsonrpc_core::types as jsonrpc_type;
use jsonrpc_pubsub::PubSubHandler;
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use ::*;
use ::error::*;
use ::jsonrpc::*;



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


    // TODO this should work with Json::Value in general instead of String
    pub fn dispatch<C>(&self, sock_path: &PathBuf, codec: C) -> AsyncResult<(),Error>
        where C: 'static + Decoder<Item=String, Error=::std::io::Error>
                 + Clone + Encoder<Item=String, Error=::std::io::Error>
    {
        let sock = UnixListener::bind(sock_path, &self.handle).unwrap();

        println!("listening on {:?}", sock_path);

        let this = self.clone();
        let server_fut = sock.incoming().for_each( move |(connection, peer_addr)|
        {
            let peer_credentials = connection.peer_cred();
            //let framed = LinesCodec::new().framed(connection);
            let framed = Framed::new( connection, codec.clone() );
            let (sink, stream) = framed.split();

            let client_fut = this.serve_client(sink, stream);
            this.handle.spawn(client_fut.map_err( |e| warn!("Serving client failed: {}", e) ) );
            Ok( () )
        } );

        Box::new( server_fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )
    }


    pub fn serve_client<O,I>(&self, sink: O, stream: I) -> AsyncResult<(), ::std::io::Error>
        where O: 'static + Sink<SinkItem=String, SinkError=::std::io::Error>,
              I: 'static + Stream<Item=String, Error=::std::io::Error>
    {
        let this = self.clone();

        // TODO use channel to order responses from multiple clients
        // let (resp_tx, resp_rx) = mpsc::channel(CHANNEL_CAPACITY);
        let client_fut = stream.for_each( move |line|
        {
            println!("got line: {}", line);
//            let request = serde_json::from_str::<jsonrpc_type::Request>(&line)?;
//                // TODO .map_err( |e| e.context( ErrorKind::TODO.into() ).into() )?;
//            println!("got request: {:?}", request);

            this.jsonrpc_dispatch.handle_request(&line)
                .then( |res|
                {
                    match res {
                        Ok(val) => println!("Got result {:?}", val),
                        Err(e)  => println!("Got error {:?}", e),
                    };
                    Ok( () )
                } )

//            let response_json = json!({"jsonrpc": request.jsonrpc, "id": request.id, "result": "true"});
//            println!("sending response: {}", response_json);
        } );
        Box::new(client_fut)
    }
}
