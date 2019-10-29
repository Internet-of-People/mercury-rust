use async_trait::async_trait;
use failure::{err_msg, Fallible};

use crate::dapp::user_interactor::{DAppAction, UserInteractor};
use did::model::ProfileId;
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

#[async_trait]
impl UserInteractor for FakeUserInteractor {
    async fn initialize(&self) -> Fallible<()> {
        Ok(())
    }

    async fn confirm_dappaction(&self, _action: &DAppAction) -> Fallible<()> {
        Ok(())
    }

    async fn confirm_pairing(&self, _request: &RelationHalfProof) -> Fallible<()> {
        Ok(())
    }

    async fn notify_pairing(&self, _response: &RelationProof) -> Fallible<()> {
        Ok(())
    }

    async fn select_profile(&self) -> Fallible<ProfileId> {
        let profile_id_res =
            self.active_profile.to_owned().ok_or(err_msg("No profile was selected"));
        profile_id_res
    }
}
