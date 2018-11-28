use std::rc::Rc;
use std::path::PathBuf;

use failure::Fail;
use futures::prelude::*;
use tokio_codec::{Decoder, Encoder};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use ::error::*;
use ::service::*;



pub mod server_dispatcher;



pub struct UdsServer
{
    listener:   UnixListener,
    handle:     reactor::Handle,
}

impl UdsServer
{
    pub fn new(sock_path: &PathBuf, handle: reactor::Handle)
        -> Result<Self, ::std::io::Error>
    {
        // TODO force deleting socket file when dropped so the code can start again without manual intervention
        let listener = UnixListener::bind(sock_path, &handle)?;
        debug!("listening on {:?}", sock_path);
        Ok( Self{listener, handle} )
    }


    pub fn dispatch<C>(self, codec: C, service: Rc<ConnectService>) -> AsyncResult<(),Error>
        where C: 'static + Decoder<Item=String, Error=::std::io::Error> +
                   Clone + Encoder<Item=String, Error=::std::io::Error>
    {
        let rpc_server = server_dispatcher::JsonRpcServer::new(service);

        let handle = self.handle; // NOTE convince the borrow checker about this partial moved field
        let server_fut = self.listener.incoming().for_each( move |(connection, _peer_addr)|
        {
            let _peer_credentials = connection.peer_cred();

            let client_fut = rpc_server.serve_duplex_stream( connection, codec.clone() );
            handle.spawn( client_fut.map_err( |e| warn!("Serving client failed: {}", e) ) );
            Ok( () )
        } );

        Box::new( server_fut.map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() ) )
    }
}
