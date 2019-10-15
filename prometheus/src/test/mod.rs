use failure::err_msg;
use futures::IntoFuture;

use crate::dapp::user_interactor::{DAppAction, UserInteractor};
use did::model::{AsyncFallible, ProfileId};
use mercury_home_protocol::{RelationHalfProof, RelationProof};

pub struct FakeUserInteractor {
    active_profile: Option<ProfileId>,
}

impl FakeUserInteractor {
    pub fn new() -> Self {
        Self { active_profile: Default::default() }
    }

    pub fn set_active_profile(&mut self, profile_id: Option<ProfileId>) {
        self.active_profile = profile_id;
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
        let profile_id = self.active_profile.to_owned().ok_or(err_msg("No profile was selected"));
        Box::new(profile_id.into_future())
    }
}
