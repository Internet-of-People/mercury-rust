use std::rc::Rc;

use futures::{future::ok, Stream};

use crate::dapp::dapp_session::*;
use did::model::ProfileId;
use keyvault::multicipher::MKeyId;
use mercury_home_protocol::{ApplicationId, AsyncFallible};

pub struct ServiceClient {}

impl ServiceClient {
    pub fn new() -> Self {
        Self {}
    }
}

impl DAppSessionService for ServiceClient {
    fn dapp_session(&self, app: ApplicationId) -> AsyncFallible<Rc<dyn DAppSession>> {
        // TODO
        Box::new(ok(Rc::new(SessionClient {}) as Rc<dyn DAppSession>))
    }
}

pub struct SessionClient {}

impl DAppSession for SessionClient {
    fn dapp_id(&self) -> &ApplicationId {
        unimplemented!()
    }

    fn selected_profile(&self) -> &MKeyId {
        unimplemented!()
    }

    fn relations(&self) -> AsyncFallible<Vec<Box<dyn Relation>>> {
        unimplemented!()
    }

    fn relation(&self, id: &MKeyId) -> AsyncFallible<Option<Box<dyn Relation>>> {
        unimplemented!()
    }

    fn initiate_relation(&self, with_profile: &ProfileId) -> AsyncFallible<()> {
        unimplemented!()
    }

    fn checkin(&self) -> AsyncFallible<Box<dyn Stream<Item = DAppEvent, Error = ()>>> {
        unimplemented!()
    }
}
