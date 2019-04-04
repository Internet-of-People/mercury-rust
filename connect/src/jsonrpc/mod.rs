use std::rc::Rc;
use std::path::PathBuf;

use failure::Fail;
use futures::prelude::*;
use log::*;
use tokio_codec::{Decoder, Encoder};
use tokio_core::reactor;
use tokio_uds::UnixListener;

use mercury_home_protocol::*;
use crate::error::*;
use crate::service::*;



pub mod api;
pub mod client_proxy;
pub mod server_dispatcher;



pub struct UdsServer
{
    path:       PathBuf,
    //listener:   UnixListener,
    handle:     reactor::Handle,
}

impl UdsServer
{
    pub fn new(sock_path: &PathBuf, handle: reactor::Handle)
        -> Result<Self, ::std::io::Error>
    {
//        let listener = UnixListener::bind(sock_path, &handle)?;
//        debug!("listening on {:?}", sock_path);
        Ok( Self{ path: sock_path.clone(), handle } )
    }


    pub fn dispatch<C>(&self, codec: C, service: Rc<ConnectService>)
        -> AsyncResult<(), Error>
    where C: 'static + Decoder<Item=String, Error=::std::io::Error> +
               Clone + Encoder<Item=String, Error=::std::io::Error>
    {
        let listener = match UnixListener::bind(&self.path, &self.handle) {
            Ok(sock) => sock,
            Err(e) => {
                let err = Err( e.context( ErrorKind::ConnectionFailed.into() ).into() );
                return Box::new( err.into_future() ) // as AsyncResult<(), Error> )
            },
        };
        debug!("listening on {:?}", self.path);

        let rpc_server = server_dispatcher::JsonRpcServer::new( service, self.handle.clone() );

        let handle = self.handle.clone(); // NOTE convince the borrow checker about this partial moved field
        let server_fut = listener.incoming().for_each( move |(connection, _peer_addr)|
        {
            let _peer_credentials = connection.peer_cred();

            let client_fut = rpc_server.serve_duplex_stream( connection, codec.clone() );
            handle.spawn( client_fut.map_err( |e| warn!("Serving client failed: {}", e) ) );
            Ok( () )
        } )
        .map_err( |e| e.context( ErrorKind::ImplementationError.into() ).into() );

        Box::new(server_fut)
    }
}


impl Drop for UdsServer {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.path) {
            Ok(()) => debug!("Cleaned up socket file {:?}", self.path),
            Err(_e) => warn!("Failed to clean up socket file {:?}", self.path),
        }
    }
}