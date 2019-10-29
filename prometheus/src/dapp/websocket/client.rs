use std::sync::Arc;

use async_trait::async_trait;
use failure::Fallible;

use crate::dapp::dapp_session::*;
use did::model::ProfileId;
use keyvault::multicipher::MKeyId;
use mercury_home_protocol::ApplicationId;

pub struct ServiceClient {}

impl ServiceClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl DAppSessionService for ServiceClient {
    async fn dapp_session(&self, _app: ApplicationId) -> Fallible<Arc<dyn DAppSession>> {
        // TODO
        Ok(Arc::new(SessionClient {}) as Arc<dyn DAppSession>)
    }
}

pub struct SessionClient {}

#[async_trait]
impl DAppSession for SessionClient {
    fn dapp_id(&self) -> &ApplicationId {
        unimplemented!()
    }

    fn profile_id(&self) -> &MKeyId {
        unimplemented!()
    }

    async fn relations(&self) -> Fallible<Vec<Box<dyn Relation>>> {
        unimplemented!()
    }

    async fn relation(&self, _id: &MKeyId) -> Fallible<Option<Box<dyn Relation>>> {
        unimplemented!()
    }

    async fn initiate_relation(&self, _with_profile: &ProfileId) -> Fallible<()> {
        unimplemented!()
    }

    async fn checkin(&self) -> Fallible<DAppEventStream> {
        unimplemented!()
    }
}
