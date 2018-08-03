use std::net::{SocketAddr, IpAddr};

use futures::{future, Future};
use multiaddr::{Multiaddr, AddrComponent};
use tokio_core::reactor;
use tokio_core::net::TcpStream;

use super::*;



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
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("StunTurnTcpConnector.connect "))) )
    }
}



pub struct TcpHomeConnector
{
    // TODO
}


impl HomeConnector for TcpHomeConnector
{
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        // TODO in case of TCP addresses, use StunTurnTcpConnector to build an async TcpStream
        //      to it and build a Home proxy on top of it
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("TcpHomeConnector.connect "))) )
    }
}



/// Convert a TCP/IP multiaddr to a SocketAddr. For multiaddr instances that are not TCP or IP, error is returned.
pub fn multiaddr_to_socketaddr(multiaddr: &Multiaddr) -> Result<SocketAddr, ErrorToBeSpecified>
{
    let mut components = multiaddr.iter();

    let ip_address = match components.next()
        {
            Some( AddrComponent::IP4(address) ) => IpAddr::from(address),
            Some( AddrComponent::IP6(address) ) => IpAddr::from(address),
            _ => return Err(ErrorToBeSpecified::TODO(String::from("multiaddr_to_socketaddr fails at ip "))),
        };

    let ip_port = match components.next()
        {
            Some( AddrComponent::TCP(port) ) => port,
            Some( AddrComponent::UDP(port) ) => port,
            _ => return Err(ErrorToBeSpecified::TODO(String::from("multiaddr_to_socketaddr fails at port "))),
        };

    Ok( SocketAddr::new(ip_address, ip_port) )
}



pub struct SimpleTcpHomeConnector
{
    handle: reactor::Handle,
}


impl SimpleTcpHomeConnector
{
    pub fn new(handle: reactor::Handle) -> Self
        { Self{ handle: handle} }

    pub fn connect_addr(addr: &Multiaddr, handle: &reactor::Handle) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        // TODO handle other multiaddresses, not only TCP
        let tcp_addr = match multiaddr_to_socketaddr(addr)
        {
            Ok(res) => res,
            Err(_) => return Box::new( future::err(ErrorToBeSpecified::TODO(String::from("SimpleTcpHomeConnector.connect_addr fails at multiaddr_to_socketaddr "))) )
        };

        let tcp_str = TcpStream::connect(&tcp_addr, handle)
            .map_err( |_| ErrorToBeSpecified::TODO(String::from("SimpleTcpHomeConnector.connect_addr fails at TcpStream::connect")) );
        Box::new(tcp_str)
    }
}


impl HomeConnector for SimpleTcpHomeConnector
{
    fn connect(&self, home_profile: &Profile, signer: Rc<Signer>) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let addrs = match home_profile.facet {
            ProfileFacet::Home(ref home_facet) => home_facet.addrs.clone(),
            _ => return Box::new(future::err(ErrorToBeSpecified::TODO("connect: not a home profile".to_owned()))),
        };

        let handle_clone = self.handle.clone();
        let tcp_conns = addrs.iter().map( move |addr| SimpleTcpHomeConnector::connect_addr(&addr, &handle_clone) );

        let handle_clone = self.handle.clone();
        let capnp_home = future::select_ok(tcp_conns)
            .and_then( move |(tcp_stream, _pending_futs)|
            {
                use mercury_home_protocol::handshake::temp_tcp_handshake_until_tls_is_implemented;

                temp_tcp_handshake_until_tls_is_implemented(tcp_stream, signer)
            }).map( |(reader, writer, peer_ctx)| {
                use protocol_capnp::HomeClientCapnProto;
                Rc::new( HomeClientCapnProto::new(reader, writer, peer_ctx, handle_clone) ) as Rc<Home>
            });
        Box::new(capnp_home)
    }
}



#[cfg(test)]
mod tests
{
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};


    #[test]
    fn test_multiaddr_conversion()
    {
        let multiaddr = "/ip4/127.0.0.1/tcp/22".parse::<Multiaddr>().unwrap();
        let socketaddr = multiaddr_to_socketaddr(&multiaddr).unwrap();
        assert_eq!(socketaddr, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 22));

        let multiaddr = "/ip4/127.0.0.1/utp".parse::<Multiaddr>().unwrap();
        let socketaddr = multiaddr_to_socketaddr(&multiaddr);
        assert_eq!(socketaddr, Result::Err(ErrorToBeSpecified::TODO(String::from("multiaddr_to_socketaddr fails at port "))));
    }
}