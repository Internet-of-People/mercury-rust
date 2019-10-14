use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;

use failure::{bail, format_err, Fail};
use futures::{future, Future, IntoFuture};
use log::*;
use multiaddr::{AddrComponent, Multiaddr};
use tokio::net::tcp::TcpStream;

use crate::*;
use mercury_home_protocol::net::HomeConnector;
use mercury_home_protocol::primitives::FacetExtractor;
use mercury_home_protocol::{AsyncFallible, AsyncResult, Home, Profile, ProfileId, Signer};
use std::collections::HashMap;

/// Convert a TCP/IP multiaddr to a SocketAddr. For multiaddr instances that are not TCP or IP, error is returned.
pub fn multiaddr_to_socketaddr(multiaddr: &Multiaddr) -> Fallible<SocketAddr> {
    let mut components = multiaddr.iter();

    let ip_address = match components.next() {
        Some(AddrComponent::IP4(address)) => IpAddr::from(address),
        Some(AddrComponent::IP6(address)) => IpAddr::from(address),
        Some(comp) => bail!("Multiaddress component type not supported: {}", comp),
        _ => bail!("No component found in multiaddress"),
    };

    let ip_port = match components.next() {
        Some(AddrComponent::TCP(port)) => port,
        Some(AddrComponent::UDP(port)) => port,
        comp => {
            bail!("TCP or UDP address components are supported after IP, got {:?} instead", comp)
        }
    };

    Ok(SocketAddr::new(ip_address, ip_port))
}

pub struct TcpHomeConnector {
    profile_repo: Rc<RefCell<dyn DistributedPublicProfileRepository>>,
    // TODO consider tearing down and rebuilding the whole connection in case of a network error
    // TODO change key to pair(persona_profile_id, home_profile_id) instead on the long term
    addr_cache: Rc<RefCell<HashMap<Multiaddr, Rc<dyn Home>>>>,
}

impl TcpHomeConnector {
    pub fn new(profile_repo: Rc<RefCell<dyn DistributedPublicProfileRepository>>) -> Self {
        Self { profile_repo, addr_cache: Default::default() }
    }

    pub fn connect_addr(addr: &Multiaddr) -> AsyncFallible<TcpStream> {
        // TODO handle other multiaddresses, not only TCP
        let tcp_addr = match multiaddr_to_socketaddr(addr) {
            Ok(res) => res,
            Err(err) => return Box::new(future::err(err)),
        };

        debug!("Connecting to socket address {}", tcp_addr);
        let tcp_str = TcpStream::connect(&tcp_addr)
            .map_err(|e| format_err!("Failed to connect to home node: {}", e));
        Box::new(tcp_str)
    }

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
        let tcp_conns = addresses.iter().map(move |addr| {
            let addr_clone = addr.to_owned();
            TcpHomeConnector::connect_addr(&addr)
                .map_err(|err| {
                    err.context(mercury_home_protocol::error::ErrorKind::ConnectionToHomeFailed)
                        .into()
                })
                .map(move |tcp_stream| (addr_clone, tcp_stream))
        });

        let addr_cache_clone = self.addr_cache.clone();
        let capnp_home = future::select_ok(tcp_conns)
            .and_then( move |((addr, tcp_stream), _pending_futs)|
            {
                use mercury_home_protocol::handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done;
                temporary_unsafe_tcp_handshake_until_diffie_hellman_done(tcp_stream, signer)
                    .map_err(|err| err.context(mercury_home_protocol::error::ErrorKind::DiffieHellmanHandshakeFailed).into())
                    .map( move |(reader, writer, _peer_ctx)| {
                        use mercury_home_protocol::mercury_capnp::client_proxy::HomeClientCapnProto;
                        let home = Rc::new( HomeClientCapnProto::new(reader, writer) ) as Rc<dyn Home>;
                        debug!("Save home {:?} client into cache for reuse", addr);
                        addr_cache_clone.borrow_mut().insert(addr, home.clone());
                        home
                    })
            });

        Box::new(capnp_home)
    }

    fn connect_to_home_profile(
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

impl HomeConnector for TcpHomeConnector {
    fn connect(
        self: Rc<Self>,
        home_profile_id: &ProfileId,
        addr_hint: Option<Multiaddr>,
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, mercury_home_protocol::error::Error> {
        // TODO logic should first try connecting with addr_hint first, then if failed,
        //      also try the full home profile loading and connecting path as well
        if let Some(addr) = addr_hint {
            return Box::new(self.connect_to_addrs(&[addr], signer));
        }

        use mercury_home_protocol::error::ErrorKind;
        let this = self.clone();
        let home_conn_fut = self
            .profile_repo
            .borrow()
            .get_public(home_profile_id)
            .inspect(move |home_profile| {
                debug!("Finished loading details for home {}", home_profile.id())
            })
            .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            .and_then(move |home_profile| {
                this.connect_to_home_profile(&home_profile, signer)
                    .map_err(|err| err.context(ErrorKind::ConnectionToHomeFailed).into())
            });
        Box::new(home_conn_fut)
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
        assert!(socketaddr.is_err());
    }
}
