use std::rc::Rc;

use multiaddr::Multiaddr;

use crate::*;

pub trait HomeConnector {
    fn connect(
        self: Rc<Self>,
        home_profile_id: &ProfileId,
        addr_hint: Option<Multiaddr>,
        signer: Rc<dyn Signer>,
    ) -> AsyncResult<Rc<dyn Home>, Error>;
}
