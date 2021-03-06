use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;

use failure::Fail;
use futures::{future, Future};
use log::*;
use multiaddr::{AddrComponent, Multiaddr};
use tokio_core::net::TcpStream;
use tokio_core::reactor;

use crate::*;
use mercury_home_protocol::net::HomeConnector;
use std::collections::HashMap;

/// Convert a TCP/IP multiaddr to a SocketAddr. For multiaddr instances that are not TCP or IP, error is returned.
pub fn multiaddr_to_socketaddr(multiaddr: &Multiaddr) -> Result<SocketAddr, Error> {
    let mut components = multiaddr.iter();

    let ip_address = match components.next() {
        Some(AddrComponent::IP4(address)) => IpAddr::from(address),
        Some(AddrComponent::IP6(address)) => IpAddr::from(address),
        _ => Err(ErrorKind::AddressConversionFailed)?,
    };

    let ip_port = match components.next() {
        Some(AddrComponent::TCP(port)) => port,
        Some(AddrComponent::UDP(port)) => port,
        _ => return Err(ErrorKind::AddressConversionFailed)?,
    };

    Ok(SocketAddr::new(ip_address, ip_port))
}

pub struct SimpleTcpHomeConnector {
    handle: reactor::Handle,
    // TODO consider tearing down and rebuilding the whole connection in case of a network error
    addr_cache: Rc<RefCell<HashMap<Multiaddr, Rc<dyn Home>>>>,
}

impl SimpleTcpHomeConnector {
    pub fn new(handle: reactor::Handle) -> Self {
        Self { handle, addr_cache: Default::default() }
    }

    pub fn connect_addr(
        addr: &Multiaddr,
        handle: &reactor::Handle,
    ) -> AsyncResult<TcpStream, Error> {
        // TODO handle other multiaddresses, not only TCP
        let tcp_addr = match multiaddr_to_socketaddr(addr) {
            Ok(res) => res,
            Err(err) => return Box::new(future::err(err)),
        };

        debug!("Connecting to socket address {}", tcp_addr);
        let tcp_str = TcpStream::connect(&tcp_addr, handle)
            .map_err(|err| err.context(ErrorKind::ConnectionFailed).into());
        Box::new(tcp_str)
    }
}

impl HomeConnector for SimpleTcpHomeConnector {
    fn connect_to_addrs(
        &self,
        addresses: &[Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, mercury_home_protocol::error::Error> {
        let first_cached =
            addresses.iter().filter_map(|addr| self.addr_cache.borrow().get(addr).cloned()).next();
        if let Some(home) = first_cached {
            debug!("Home address {:?} found in cache, reuse connection", addresses);
            return Box::new(Ok(home).into_future());
        }

        debug!("Home address {:?} not found in cache, connecting", addresses);
        let handle_clone = self.handle.clone();
        let tcp_conns = addresses.iter().map(move |addr| {
            let addr_clone = addr.to_owned();
            SimpleTcpHomeConnector::connect_addr(&addr, &handle_clone)
                .map_err(|err| {
                    err.context(mercury_home_protocol::error::ErrorKind::ConnectionToHomeFailed)
                        .into()
                })
                .map(move |tcp_stream| (addr_clone, tcp_stream))
        });

        let handle_clone = self.handle.clone();
        let addr_cache_clone = self.addr_cache.clone();
        let capnp_home = future::select_ok(tcp_conns)
            .and_then( move |((addr, tcp_stream), _pending_futs)|
            {
                use mercury_home_protocol::handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done;
                temporary_unsafe_tcp_handshake_until_diffie_hellman_done(tcp_stream, signer)
                    .map_err(|err| err.context(mercury_home_protocol::error::ErrorKind::DiffieHellmanHandshakeFailed).into())
                    .map( move |(reader, writer, _peer_ctx)| {
                        use mercury_home_protocol::mercury_capnp::client_proxy::HomeClientCapnProto;
                        let home = Rc::new( HomeClientCapnProto::new(reader, writer, handle_clone) ) as Rc<dyn Home>;
                        debug!("Save home {:?} client into cache for reuse", addr);
                        addr_cache_clone.borrow_mut().insert(addr, home.clone());
                        home
                    })
            });

        Box::new(capnp_home)
    }

    fn connect_to_home(
        &self,
        home_profile: &Profile,
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, mercury_home_protocol::error::Error> {
        let addrs = match home_profile.as_home() {
            Some(ref home_facet) => home_facet.addrs.clone(),
            None => {
                return Box::new(future::err(
                    mercury_home_protocol::error::ErrorKind::ProfileMismatch.into(),
                ));
            }
        };

        self.connect_to_addrs(&addrs, signer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_multiaddr_conversion() {
        let multiaddr = "/ip4/127.0.0.1/tcp/22".parse::<Multiaddr>().unwrap();
        let socketaddr = multiaddr_to_socketaddr(&multiaddr).unwrap();
        assert_eq!(socketaddr, SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 22));

        let multiaddr = "/ip4/127.0.0.1/utp".parse::<Multiaddr>().unwrap();
        let socketaddr = multiaddr_to_socketaddr(&multiaddr);

        assert_eq!(socketaddr, Result::Err(ErrorKind::AddressConversionFailed.into()));
    }
}
