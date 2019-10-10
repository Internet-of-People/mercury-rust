use std::rc::Rc;

use failure::{err_msg, Fallible};
use futures::IntoFuture;

use crate::dapp::user_interactor::{DAppAction, UserInteractor};
use did::model::{AsyncFallible, ProfileId};
use did::vault::ProfileVault;
use mercury_home_protocol::{RelationHalfProof, RelationProof};

pub struct FakeUserInteractor {
    profile_vault: Rc<dyn ProfileVault>,
}

impl FakeUserInteractor {
    pub fn new(profile_vault: Rc<dyn ProfileVault>) -> Self {
        Self { profile_vault }
    }

    fn select_profile_sync(&self) -> Fallible<ProfileId> {
        self.profile_vault.get_active()?.ok_or(err_msg("No default active profile selected"))
    }
}

impl UserInteractor for FakeUserInteractor {
    fn initialize(&self) -> AsyncFallible<()> {
        Box::new(Ok(()).into_future())
    }

    fn confirm_dappaction(&self, _action: &DAppAction) -> AsyncFallible<()> {
        Box::new(Ok(()).into_future())
    }

    fn confirm_pairing(&self, _request: &RelationHalfProof) -> AsyncFallible<()> {
        Box::new(Ok(()).into_future())
    }

    fn notify_pairing(&self, _response: &RelationProof) -> AsyncFallible<()> {
        Box::new(Ok(()).into_future())
    }

    fn select_profile(&self) -> AsyncFallible<ProfileId> {
        Box::new(self.select_profile_sync().into_future())
    }
}
