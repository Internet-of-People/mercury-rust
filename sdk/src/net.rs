use std::net::{SocketAddr};

use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr, Protocol};
use tokio_core::reactor;
use tokio_core::net::{TcpStream, TcpStreamNew};

use super::*;



// TODO this should return simply Rc<Home> but then it's a lot of boilerplate to compile until implemented
pub fn tcp_home(tcp_stream: TcpStream) -> Option<Rc<Home>>
{
    // TODO wrap a Tcp stream into a Home implementation, hopefully using generated Capnproto code
    None
}



pub struct StunTurnTcpConnector
{
    // TODO
}


impl StunTurnTcpConnector
{
    pub fn connect(&self, addr: &SocketAddr) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        // TODO
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}



pub struct TcpHomeConnector
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



pub struct SimpleTcpHomeConnector
{
    handle: reactor::Handle,
}


impl SimpleTcpHomeConnector
{
    pub fn connect(addr: &Multiaddr, handle: &reactor::Handle) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        if ! addr.protocol().contains(&Protocol::TCP)
            { return Box::new( future::err(ErrorToBeSpecified::TODO) ); }

        // TODO how to extract TCPv4/v6 address???
        return Box::new( future::err(ErrorToBeSpecified::TODO) );
//        TcpStream::connect(tcp_addr, &handle)
    }

}


impl HomeConnector for SimpleTcpHomeConnector
{
    fn connect(&self, home_profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let handle_clone = self.handle.clone();
        let tcp_conns = home_profile.facets.iter()
            .flat_map( |facet|
                match facet {
                    &ProfileFacet::Home(ref home) => home.addrs.clone(),
                    _ => Vec::new()
                }
            )
            .map(  move |addr| SimpleTcpHomeConnector::connect(&addr, &handle_clone) );

        let tcp_home = future::select_ok(tcp_conns)
            // TODO remove unwrap() when tcp_home() signature is fixed
            .map( |(tcp, _pending_futs)| tcp_home(tcp).unwrap() );
        Box::new(tcp_home)
    }
}
