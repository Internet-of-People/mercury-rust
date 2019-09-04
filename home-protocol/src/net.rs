use std::rc::Rc;

use multiaddr::Multiaddr;

use crate::*;

pub trait HomeConnector {
    fn connect_to_addrs(
        &self,
        addresses: &[Multiaddr],
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, Error>;

    /// Initiate a permanent connection to the home server defined by `home_profile`, or return an
    /// existing, live `Home` immediately.
    /// `home_profile` must have a HomeFacet with at least an address filled in.
    /// `signer` belongs to me.
    fn connect_to_home(
        &self,
        home_profile: &Profile,
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, Error>;
}
