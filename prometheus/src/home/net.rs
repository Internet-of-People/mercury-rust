use std::cell::RefCell;
use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use failure::{bail, format_err};
use futures::future;
use log::*;
use multiaddr::{AddrComponent, Multiaddr};
use tokio::net::tcp::TcpStream;

use crate::*;
use mercury_home_protocol::primitives::ProfileFacets;
use mercury_home_protocol::{Home, Profile, ProfileId, Signer};

#[async_trait(?Send)]
pub trait HomeConnector {
    async fn connect<'s, 'p>(
        &'s mut self,
        home_profile_id: &'p ProfileId,
        addr_hints: &'p [Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> Fallible<Rc<dyn Home + 's>>
    where
        's: 'p;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Endpoint {
    Tcp(SocketAddr),
    Udp(SocketAddr),
}

/// Convert a TCP/IP multiaddr to a SocketAddr. For multiaddr instances that are not TCP or IP, error is returned.
pub fn multiaddr_to_endpoint(multiaddr: &Multiaddr) -> Fallible<Endpoint> {
    let mut components = multiaddr.iter();

    let ip_address = match components.next() {
        Some(AddrComponent::IP4(address)) => IpAddr::from(address),
        Some(AddrComponent::IP6(address)) => IpAddr::from(address),
        Some(comp) => bail!("Multiaddress component type not supported: {}", comp),
        _ => bail!("No component found in multiaddress"),
    };

    let addr = |ip_port| SocketAddr::new(ip_address, ip_port);

    let endpoint = match components.next() {
        Some(AddrComponent::TCP(port)) => Endpoint::Tcp(addr(port)),
        Some(AddrComponent::UDP(port)) => Endpoint::Udp(addr(port)),
        comp => {
            bail!("TCP or UDP address components are supported after IP, got {:?} instead", comp)
        }
    };

    Ok(endpoint)
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

    pub async fn connect_addr<'a>(addr: &'a Multiaddr) -> Fallible<TcpStream> {
        // TODO handle other multiaddresses, not only TCP
        let endpoint = multiaddr_to_endpoint(addr)?;
        let tcp_addr = match endpoint {
            Endpoint::Tcp(addr) => Ok(addr),
            Endpoint::Udp(addr) => Err(format_err!("UDP {} is unsupported", addr)),
        }?;

        debug!("Connecting to TCP {}", tcp_addr);
        let tcp_res = TcpStream::connect(&tcp_addr)
            .await
            .map_err(|e| format_err!("Failed to connect to home node: {}", e));

        tcp_res
    }

    async fn connect_to_addrs<'a, 'b>(
        &'a self,
        home_profile_id: &'b ProfileId,
        addresses: &'b [Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> Fallible<Rc<dyn Home + 'a>>
    where
        'a: 'b,
    {
        let key = (signer.profile_id().to_owned(), home_profile_id.to_owned());
        let cache_hit = HOME_CACHE.with(|cache| cache.borrow().get(&key).cloned());
        if let Some((addr, home)) = cache_hit {
            debug!(
                "Home {} with address {:?} found in cache, reusing connection",
                home_profile_id, addr
            );
            return Ok(home);
        }

        debug!("Home address {:?} not found in cache, connecting", addresses);
        let tcp_conns = addresses.iter().cloned().map(move |addr| {
            Box::pin(async {
                let tcp_stream = TcpHomeConnector::connect_addr(&addr).await?;
                Fallible::Ok((addr, tcp_stream))
            })
        });

        // TODO We have to find the first successful connection **that could authenticate itself as a home node
        //      that has the given ProfileId**. At the moment the first successful TCP connection already wins.
        let ((addr, tcp_stream), _pending_futs) = future::select_ok(tcp_conns).await?;

        use mercury_home_protocol::handshake::temporary_unsafe_tcp_handshake_until_diffie_hellman_done as handshake;
        let (peer_ctx, reader, writer) = handshake(tcp_stream, signer.clone()).await?;
        use mercury_home_protocol::mercury_capnp::client_proxy::HomeClientCapnProto;
        let home = Rc::new(HomeClientCapnProto::new(peer_ctx, reader, writer)) as Rc<dyn Home>;

        debug!("Save home {:?} client into cache for reuse", addr);
        HOME_CACHE.with(|cache| {
            cache.borrow_mut().insert(
                (signer.profile_id().to_owned(), home_profile_id.to_owned()),
                (addr, home.clone()),
            );
        });

        Ok(home)
    }

    async fn connect_to_home_profile<'a, 'b>(
        &'a self,
        home_profile: &'b Profile,
        signer: Rc<dyn Signer>,
    ) -> Fallible<Rc<dyn Home + 'a>>
    where
        'a: 'b,
    {
        let addrs = match home_profile.to_home() {
            Some(ref home_facet) => home_facet.addrs.clone(),
            None => {
                return Err(err_msg("Profile is not a Home node"));
            }
        };

        self.connect_to_addrs(&home_profile.id(), &addrs, signer).await
    }
}

#[async_trait(?Send)]
impl HomeConnector for TcpHomeConnector {
    async fn connect<'s, 'p>(
        &'s mut self,
        home_profile_id: &'p ProfileId,
        addr_hints: &'p [Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> Fallible<Rc<dyn Home + 's>>
    where
        's: 'p,
    {
        // TODO logic should first try connecting with addr_hint first, then if failed,
        //      also try the full home profile loading and connecting path as well
        if !addr_hints.is_empty() {
            return self.connect_to_addrs(home_profile_id, addr_hints, signer).await;
        }

        let profile_repo = match self.profile_repo.try_read() {
            Ok(repo) => repo,
            Err(e) => {
                error!("BUG: failed to lock profile repository: {}", e);
                unreachable!()
            }
        };
        let home_profile = profile_repo.get_public(home_profile_id).await?;

        debug!("Finished loading details for home {}", home_profile.id());
        self.connect_to_home_profile(&home_profile, signer).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_multiaddr_tcp() {
        let multiaddr = "/ip4/127.0.0.1/tcp/22".parse::<Multiaddr>().unwrap();
        let endpoint = multiaddr_to_endpoint(&multiaddr).unwrap();
        assert_eq!(
            endpoint,
            Endpoint::Tcp(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 22))
        );
    }

    #[test]
    fn test_multiaddr_udp() {
        let multiaddr = "/ip4/127.0.0.1/udp/53".parse::<Multiaddr>().unwrap();
        let endpoint = multiaddr_to_endpoint(&multiaddr).unwrap();
        assert_eq!(
            endpoint,
            Endpoint::Udp(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 53))
        );
    }

    #[test]
    fn test_multiaddr_invalid() {
        let multiaddr = "/ip4/127.0.0.1/utp".parse::<Multiaddr>().unwrap();
        let endpoint = multiaddr_to_endpoint(&multiaddr);
        assert!(endpoint.is_err());
    }
}
