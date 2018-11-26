use std::path::PathBuf;

use failure::Fail;
use futures::prelude::*;
use tokio_codec::{Decoder, Encoder};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use ::error::*;



pub mod server_dispatcher;



pub struct UdsServer
{
    listener:   UnixListener,
    handle:     reactor::Handle,
}

// TODO this should work not only over UDS socket streams, but over any AsyncRead/Write stream
impl UdsServer
{
    pub fn new(sock_path: &PathBuf, handle: reactor::Handle) -> Result<Self, ::std::io::Error>
    {
        let listener = UnixListener::bind(sock_path, &handle)?;
        debug!("listening on {:?}", sock_path);
        Ok( Self{listener, handle} )
    }


    pub fn dispatch<C>(self, codec: C) -> AsyncResult<(),Error>
        where C: 'static + Decoder<Item=String, Error=::std::io::Error> +
                   Clone + Encoder<Item=String, Error=::std::io::Error>
    {
        let handle = self.handle.clone();
        let server_fut = self.listener.incoming().for_each( move |(connection, _peer_addr)|
        {
            let _peer_credentials = connection.peer_cred();

            let rpc_server = server_dispatcher::JsonRpcServer::new( handle.clone() );
            let client_fut = rpc_server.serve_duplex_stream( connection, codec.clone() );
            // handle.spawn( client_fut.map_err( |e| warn!("Serving client failed: {}", e) ) );
            Ok( () )
        } );

        Box::new( server_fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )
    }
}
