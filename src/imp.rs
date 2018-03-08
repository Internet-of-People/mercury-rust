use std::net::{SocketAddr};

use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr};
use tokio_core::net::TcpStream;

use super::*;



struct StunTurnTcpConnector
{
    // TODO
}


impl StunTurnTcpConnector
{
    fn connect(&self, addr: &SocketAddr) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        // TODO
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}



struct TcpHomeConnector
{
    // TODO
}


impl HomeConnector for TcpHomeConnector
{
    fn connect(&self, home_profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        // TODO in case of TCP addresses, use StunTurnTcpConnector to build an async TcpStream
        //      to it and build a Home proxy on top of it
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}
