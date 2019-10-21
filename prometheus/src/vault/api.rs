use std::rc::Rc;
use std::str::FromStr;

use failure::{err_msg, Fallible};
use serde_derive::{Deserialize, Serialize};

use crate::*;
use claims::model::*;

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
pub trait VaultApi {
    fn restore_vault(&mut self, phrase: String) -> Fallible<()>;
    fn restore_all_profiles(&mut self) -> Fallible<RestoreCounts>;

    fn set_active_profile(&mut self, my_profile_id: &ProfileId) -> Fallible<()>;
    fn get_active_profile(&self) -> Fallible<Option<ProfileId>>;

    fn list_vault_records(&self) -> Fallible<Vec<ProfileVaultRecord>>;
    fn create_profile(&mut self, label: Option<ProfileLabel>) -> Fallible<ProfileVaultRecord>;
    fn get_vault_record(&self, id: Option<ProfileId>) -> Fallible<ProfileVaultRecord>;

    fn set_profile_label(
        &mut self,
        my_profile_id: Option<ProfileId>,
        label: ProfileLabel,
    ) -> Fallible<()>;

    fn get_profile_metadata(&self, my_profile_id: Option<ProfileId>) -> Fallible<ProfileMetadata>;
    fn set_profile_metadata(
        &mut self,
        my_profile_id: Option<ProfileId>,
        data: ProfileMetadata,
    ) -> Fallible<()>;

    fn get_profile_data(
        &self,
        id: Option<ProfileId>,
        repo_kind: ProfileRepositoryKind,
    ) -> Fallible<PrivateProfileData>;

    fn revert_profile(&mut self, my_profile_id: Option<ProfileId>) -> Fallible<PrivateProfileData>;
    fn publish_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<ProfileId>;
    fn restore_profile(
        &mut self,
        my_profile_id: Option<ProfileId>,
        force: bool,
    ) -> Fallible<PrivateProfileData>;

    fn set_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
        value: &AttributeValue,
    ) -> Fallible<()>;
    fn clear_attribute(
        &mut self,
        my_profile_id: Option<ProfileId>,
        key: &AttributeId,
    ) -> Fallible<()>;

    fn claims(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Claim>>;
    fn add_claim(&mut self, my_profile_id: Option<ProfileId>, claim: Claim) -> Fallible<()>;
    fn remove_claim(&mut self, my_profile_id: Option<ProfileId>, claim: ClaimId) -> Fallible<()>;
    fn sign_claim(
        &self,
        my_profile_id: Option<ProfileId>,
        claim: &SignableClaimPart,
    ) -> Fallible<ClaimProof>;
    fn add_claim_proof(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim: &ClaimId,
        proof: ClaimProof,
    ) -> Fallible<()>;
    fn license_claim(
        &mut self,
        my_profile_id: Option<ProfileId>,
        claim: ClaimId,
        // TODO audience, purpose, expiry, etc
    ) -> Fallible<ClaimLicense>;

    // NOTE links are derived as a special kind of claims. Maybe they could be removed from here on the long term.
    fn list_incoming_links(&self, my_profile_id: Option<ProfileId>) -> Fallible<Vec<Link>>;
    fn create_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<Link>;
    fn remove_link(
        &mut self,
        my_profile_id: Option<ProfileId>,
        peer_profile_id: &ProfileId,
    ) -> Fallible<()>;

    // TODO: This is related to add_claim and other calls, but does not conceptually belong here.
    fn claim_schemas(&self) -> Fallible<Rc<dyn ClaimSchemas>>;

    // TODO: This is related to did_homes and other calls, but does not conceptually belong here.
    fn homes(&self) -> Fallible<Vec<HomeNode>>;
}
