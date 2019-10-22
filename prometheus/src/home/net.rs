use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use failure::{bail, format_err, Fail};
use futures::{future, Future, IntoFuture};
use log::*;
use multiaddr::{AddrComponent, Multiaddr};
use tokio::net::tcp::TcpStream;

use crate::*;
use mercury_home_protocol::primitives::ProfileFacets;
use mercury_home_protocol::{AsyncFallible, Home, Profile, ProfileId, Signer};
use std::collections::HashMap;

pub trait HomeConnector {
    fn connect(
        self: Arc<Self>,
        home_profile_id: &ProfileId,
        addr_hints: &[Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> AsyncFallible<Rc<dyn Home>>;
}

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

// TODO consider tearing down and rebuilding the whole connection in case of a network error
// Map of pair<client_profile_id, home_profile_id> => pair<multiaddr, Home instance>
thread_local!(static HOME_CACHE: RefCell<HashMap<(ProfileId, ProfileId), (Multiaddr, Rc<dyn Home>)>> = Default::default());

pub struct TcpHomeConnector {
    profile_repo: Arc<RwLock<dyn DistributedPublicProfileRepository + Send + Sync>>,
}

impl TcpHomeConnector {
    pub fn new(
        profile_repo: Arc<RwLock<dyn DistributedPublicProfileRepository + Send + Sync>>,
    ) -> Self {
        Self { profile_repo }
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
        home_profile_id: &ProfileId,
        addresses: &[Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> AsyncFallible<Rc<dyn Home>> {
        let key = (signer.profile_id().to_owned(), home_profile_id.to_owned());
        let cache_hit = HOME_CACHE.with(|cache| cache.borrow().get(&key).cloned());
        if let Some((addr, home)) = cache_hit {
            debug!(
                "Home {} with address {:?} found in cache, reusing connection",
                home_profile_id, addr
            );
            return Box::new(Ok(home).into_future());
        }

        debug!("Home address {:?} not found in cache, connecting", addresses);
        let tcp_conns = addresses.iter().map(move |addr| {
            let addr_clone = addr.to_owned();
            TcpHomeConnector::connect_addr(&addr).map(move |tcp_stream| (addr_clone, tcp_stream))
        });

        let home_profile_id = home_profile_id.to_owned();
        // TODO We have to find the first successful connection **that could authenticate itself as a home node
        //      that has the given ProfileId**. At the moment the first successful TCP connection already wins.
        let capnp_home = future::select_ok(tcp_conns)
            .and_then( move |((addr, tcp_stream), _pending_futs)|
            {
                use mercury_home_protocol::handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done;
                temporary_unsafe_tcp_handshake_until_diffie_hellman_done(tcp_stream, signer.clone())
                    .map_err(|err| err.context(mercury_home_protocol::error::ErrorKind::DiffieHellmanHandshakeFailed).into())
                    .map( move |(reader, writer, _peer_ctx)| {
                        use mercury_home_protocol::mercury_capnp::client_proxy::HomeClientCapnProto;
                        let home = Rc::new( HomeClientCapnProto::new(reader, writer) ) as Rc<dyn Home>;

                        debug!("Save home {:?} client into cache for reuse", addr);
                        HOME_CACHE.with(|cache| {
                            cache.borrow_mut().insert((signer.profile_id().to_owned(), home_profile_id), (addr, home.clone()));
                        });
                        home
                    })
            });

        Box::new(capnp_home)
    }

    fn connect_to_home_profile(
        &self,
        home_profile: &Profile,
        signer: Rc<dyn Signer>,
    ) -> AsyncFallible<Rc<dyn Home>> {
        let addrs = match home_profile.to_home() {
            Some(ref home_facet) => home_facet.addrs.clone(),
            None => {
                return Box::new(future::err(err_msg("Profile is not a Home node")));
            }
        };

        self.connect_to_addrs(&home_profile.id(), &addrs, signer)
    }
}

impl HomeConnector for TcpHomeConnector {
    fn connect(
        self: Arc<Self>,
        home_profile_id: &ProfileId,
        addr_hints: &[Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> AsyncFallible<Rc<dyn Home>> {
        // TODO logic should first try connecting with addr_hint first, then if failed,
        //      also try the full home profile loading and connecting path as well
        if !addr_hints.is_empty() {
            return self.connect_to_addrs(home_profile_id, addr_hints, signer);
        }

        let profile_repo = match self.profile_repo.try_read() {
            Ok(repo) => repo,
            Err(e) => {
                error!("BUG: failed to lock profile repository: {}", e);
                unreachable!()
            }
        };
        let this = self.clone();
        let home_conn_fut = profile_repo
            .get_public(home_profile_id)
            .inspect(move |home_profile| {
                debug!("Finished loading details for home {}", home_profile.id())
            })
            .and_then(move |home_profile| this.connect_to_home_profile(&home_profile, signer));
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
