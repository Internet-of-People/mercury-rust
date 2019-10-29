use std::rc::Rc;
use std::str::FromStr;

use async_trait::async_trait;
use failure::{err_msg, Fallible};
use serde_derive::{Deserialize, Serialize};

use crate::daemon::NetworkState;
use crate::*;
use claims::model::*;
use multiaddr::Multiaddr;

#[derive(Clone, Debug, Eq, Hash, PartialEq, PartialOrd)]
pub enum ProfileRepositoryKind {
    Local,
    Base,
    Remote, // TODO Differentiate several remotes, e.g. by including a network address here like Remote(addr)
}

impl FromStr for ProfileRepositoryKind {
    type Err = failure::Error;
    fn from_str(src: &str) -> Result<Self, Self::Err> {
        match src {
            "local" => Ok(ProfileRepositoryKind::Local),
            "base" => Ok(ProfileRepositoryKind::Base),
            "remote" => Ok(ProfileRepositoryKind::Remote),
            _ => Err(err_msg("Invalid profile repository kind")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct RestoreCounts {
    pub try_count: u32,
    pub restore_count: u32,
}

// TODO expose repository synced/unsynced state of profile here
// TODO error handling better suited for HTTP status codes (analogue to checked/unchecked exceptions)
#[async_trait(?Send)]
pub trait VaultApi {
    async fn restore_vault(&mut self, phrase: String) -> Fallible<()>;
    async fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts>;

    async fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()>;
    async fn get_active_profile(&self) -> Fallible<Option<ProfileId>>;

    async fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>>;
    async fn create_profile(&mut self, label: Option<ProfileLabel>)
        -> Fallible<ProfileVaultRecord>;
    async fn get_vault_record(&self, id: Option<ProfileId>) -> Fallible<ProfileVaultRecord>;

    async fn set_profile_label(
        &mut self,
        my_profile_id: Option<ProfileId>,
        label: ProfileLabel,
    ) -> Fallible<()>;

    async fn get_profile_metadata(
        &self,
        my_profile_id: Option<ProfileId>,
    ) -> Fallible<ProfileMetadata>;
    async fn set_profile_metadata(
        &mut self,
        my_profile_id: Option<ProfileId>,
        data: ProfileMetadata,
    ) -> Fallible<()>;

    // TODO no translation from model to public API makes backwards compatibility fragile
    async fn get_profile_data(
        &self,
        id: Option<ProfileId>,
        repo_kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData>;

    // TODO no translation from model to public API makes backwards compatibility fragile
    async fn revert_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
    ) -> Fallible<PrivateProfileData>;
    async fn publish_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<ProfileId>;
    async fn restore_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData>;

    async fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()>;
    async fn clear_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
    ) -> Fallible<()>;

    async fn claims(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Claim>>;
    async fn add_claim(&mut self, my_profile_id: Option<ProfileId>, claim: Claim) -> Fallible<()>;
    async fn remove_claim(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim: ClaimId,
    ) -> Fallible<()>;
    async fn sign_claim(
        &self,
        my_profile_id: Option<ProfileId>,
        claim: &SignableClaimPart,
    ) -> Fallible<ClaimProof>;
    async fn add_claim_proof(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim: &ClaimId,
        proof: ClaimProof,
    ) -> Fallible<()>;
    async fn license_claim(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim: ClaimId,
        // TODO audience, purpose, expiry, etc
    ) -> Fallible<ClaimLicense>;

    // NOTE links are derived as a special kind of claims. Maybe they could be removed from here on the long term.
    async fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<Link>;
    async fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<()>;

    async fn did_homes(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<DidHomeStatus>>;
    async fn register_home<'a, 'b>(
        &'a mut self,
        my_id: Option<ProfileId>,
        home_id: &'b ProfileId,
        addr_hints: &'b [Multiaddr],
        network: &'a mut NetworkState,
    ) -> Fallible<()>;

    // TODO: This is related to add_claim and other calls, but does not conceptually belong here.
    async fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>>;
}
