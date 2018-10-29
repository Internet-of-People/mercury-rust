use std::path::PathBuf;

use failure::Fail;
use futures::{prelude::*};
use serde_json;
use tokio_codec::{Decoder, LinesCodec};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use ::error::*;



#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct JsonRpcRequest
{
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    params: serde_json::Value,
}


pub struct DAppEndpointDispatcherJsonRpc
{
    handle: reactor::Handle,
}


impl DAppEndpointDispatcherJsonRpc
{
    pub fn new(handle: reactor::Handle) -> Self
        { Self{handle} }


    pub fn serve(&self, sock_path: &PathBuf) -> AsyncResult<(),Error>
    {
        let sock = UnixListener::bind(sock_path, &self.handle).unwrap();

        println!("listening on {:?}", sock_path);

        let handle = self.handle.clone();
        let server_fut = sock.incoming().for_each( move |(connection, peer_addr)|
        {
            let peer_credentials = connection.peer_cred();
            let framed = LinesCodec::new().framed(connection);
            //let framed = Framed::new( connection, LinesCodec::new() );
            let (sink, stream) = framed.split();
            let client_fut = Self::serve_connection(sink, stream);
            handle.spawn(client_fut.map_err( |e| warn!("Serving client failed: {}", e) ) );
            Ok( () )
        } );

        Box::new( server_fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )
    }


    pub fn serve_connection<O,I>(sink: O, stream: I) -> AsyncResult<(), ::std::io::Error>
        where O: 'static + Sink<SinkItem=String, SinkError=::std::io::Error>,
              I: 'static + Stream<Item=String, Error=::std::io::Error>
    {
        let client_fut = stream.for_each( |line|
        {
            println!("got line: {}", line);
            let request = serde_json::from_str::<JsonRpcRequest>(&line)?;
                // TODO .map_err( |e| e.context( ErrorKind::TODO.into() ).into() )?;
            println!("got request: {:?}", request);

            // TODO properly process request

            let response_json = json!({"jsonrpc": request.jsonrpc, "id": request.id, "result": "true"});
            println!("sending response: {}", response_json);
            Ok( () )
        } );
        Box::new(client_fut)
    }
}


//    let jsonrpc_server_fut = loop_fn( (sock, recv_buf), move |(sock, recv_buf)|
//    {
//        sock.recv_dgram(recv_buf)
//            .and_then( |(sock, recv_buf, byte_count, peer_addr)|
//            {
//                println!("Received message of {} bytes from {}", byte_count, peer_addr);
//                let request = serde_json::from_slice::<JsonRpcRequest>( &recv_buf[..byte_count] )
//                    .unwrap_or( JsonRpcRequest{jsonrpc: Default::default(), id: serde_json::Value::Null, method: Default::default(), params: serde_json::Value::Null} ); // serde_json::Value::String("Invalid JSON".to_string())
//                println!("Parsed message: {:?}", request);
//
//                // TODO process message
//
//                let response_json = json!({"jsonrpc": "2.0", "id": 1, "result": "true"});
//                let response_buf = serde_json::to_vec(&response_json).unwrap();
//                sock.send_dgram(response_buf, peer_addr)
//                    .map( |(sock, _send_buf)| (sock, recv_buf) )
//            } )
//            .map( |(sock, recv_buf)| {
//                println!("Response sent, waiting for next message");
//                if true { Loop::Continue( (sock, recv_buf) ) }
//                else    { Loop::Break( () ) } // Help the compiler with return type of loop future
//            } )
//    } );
