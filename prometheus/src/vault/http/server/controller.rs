use std::convert::{TryFrom, TryInto};

use actix_web::{web, HttpResponse, Responder};
use failure::{err_msg, format_err, Fallible};
use futures::future::{FutureExt, TryFutureExt};
use log::*;

use crate::names::DeterministicNameGenerator;
use crate::vault::api_impl::VaultState;
use crate::*;
use claims::model::*;
use keyvault::Seed;
use multiaddr::Multiaddr;

pub fn generate_bip39_phrase() -> impl Responder {
    let phrase_str = Seed::generate_bip39();
    let words = phrase_str.split_whitespace().collect::<Vec<_>>();
    HttpResponse::Ok().json(words)
}

pub fn validate_bip39_phrase(words: web::Json<Vec<String>>) -> impl Responder {
    let phrase = words.join(" ");
    let is_valid = Seed::from_bip39(&phrase).is_ok();
    HttpResponse::Ok().json(is_valid)
}

pub fn validate_bip39_word(word: web::Json<String>) -> impl Responder {
    let is_valid = Seed::check_word(&word);
    HttpResponse::Ok().json(is_valid)
}

/// This macro moves a list of `pub async` functions into a hidden module and creates compatibility
/// wrappers into this module.
///
/// So seemingly you can develop your methods by out-commenting the macro and you get proper IDE
/// support. And after you are done with a new controller, you put the macro back and expose the
/// compatibility method to an actix Route.
///
/// ```ignore
///     mod compat {
///         pub async fn init_vault(...) {...}
///     }
///     pub fn init_vault(
///         state: web::Data<Mutex<DaemonState>>,
///         words: web::Json<Vec<String>>,
///     ) -> impl futures01::Future<Item = actix_http::Response, Error = ()> {
///         super::init_vault(state, words).unit_error().boxed_local().compat()
///     }
/// ```
macro_rules! compat {

    { $(pub async fn $n:ident ( $($p:ident : $t:ty),* $(,)? ) -> $r:ty $b:block )* } => {
        mod asynch {
            use super::*;
            $(
                pub async fn $n( $( $p: $t ),* ) -> $r $b
            )*
        }

        $(
            pub fn $n( $( $p: $t ),* ) -> impl futures01::Future<Item = actix_http::Response, Error = ()> {
                asynch::$n( $( $p ),* ).unit_error().boxed_local().compat()
            }
        )*
    }

}

compat! {

// TODO this Fallible -> Responder mapping + logging should be less manual,
//      at least parts should be generated, e.g. using macros
pub async fn init_vault(
    state: web::Data<Mutex<DaemonState>>,
    words: web::Json<Vec<String>>,
) -> HttpResponse {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    let phrase = words.join(" ");
    match state.vault.restore_vault(phrase).await {
        Ok(()) => {
            debug!("Initialized vault");
            HttpResponse::Created().body("")
        }
        Err(e) => {
            error!("Failed to initialize vault: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

pub async fn restore_all_dids(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.restore_all_profiles().await {
        Ok(counts) => {
            debug!("Restored all profiles of vault");
            // Can we expose counts directly here or should we duplicate a similar, independent data structure to be used on the web UI?
            HttpResponse::Created().json(counts)
        }
        Err(e) => {
            error!("Failed to restore all profiles of vault: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn get_default_did(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.get_active_profile().await {
        Ok(did_opt) => HttpResponse::Ok().json(did_opt.map(|did| did.to_string())),
        Err(e) => {
            error!("Failed to get default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn set_default_did(
    state: web::Data<Mutex<DaemonState>>,
    did_str: web::Json<String>,
) -> HttpResponse {
    let did = match did_str.parse::<ProfileId>() {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.set_active_profile(&did).await {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => {
            error!("Failed to set default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn list_did(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_dids_from_state(&state.vault).await {
        Ok(dids) => {
            debug!("Listing {} profiles", dids.len());
            HttpResponse::Ok().json(dids)
        }
        Err(e) => {
            error!("Failed to list profiles: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn create_did(
    state: web::Data<Mutex<DaemonState>>,
    label: web::Json<ProfileLabel>,
) -> HttpResponse {
    let mut label = label;
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match create_dids_to_state(&mut state.vault, &mut label).await {
        Ok(entry) => {
            debug!("Created profile {} with label {}", entry.id, entry.label);
            HttpResponse::Created().json(entry)
        }
        Err(e) => {
            error!("Failed to create profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn get_did(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    let entry_res = async move {
        let rec = state.vault.get_vault_record(did).await?;
        Fallible::Ok(VaultEntry::try_from(&rec)?)
    };
    match entry_res.await {
        Ok(entry) => {
            debug!("Fetched info for profile {}", &did_path);
            HttpResponse::Ok().json(entry)
        }
        Err(e) => {
            error!("Failed to get profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn restore(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    force: web::Json<bool>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.restore_profile(did, *force).await {
        Ok(priv_data) => {
            debug!("Restored profile {}", &did_path);
            HttpResponse::Ok().json(priv_data) // TODO consider security here
        }
        Err(e) => {
            error!("Failed to restore profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn revert(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.revert_profile(did).await {
        Ok(priv_data) => {
            debug!("Reverted profile {}", &did_path);
            HttpResponse::Ok().json(priv_data) // TODO consider security here
        }
        Err(e) => {
            error!("Failed to revert profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn publish(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    force: web::Json<bool>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.publish_profile(did, *force).await {
        Ok(id) => {
            debug!("Published profile {}", &did_path);
            HttpResponse::Ok().json(id)
        }
        Err(e) => {
            error!("Failed to publish profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn rename_did(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    label_arg: web::Json<ProfileLabel>,
) -> HttpResponse {
    let mut label = label_arg.to_owned();
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    let did = match did_res(&state.vault, &did_path).await {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let fut = async move {
        reset_label_if_empty(&mut state.vault, &mut label, &did).await?;
        state.vault.set_profile_label(Some(did), label.to_owned()).await?;
        Fallible::Ok(())
    };
    match fut.await {
        Ok(()) => {
            debug!("Renamed profile {} to {}", &did_path, label_arg);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to rename profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn get_profile(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.get_profile_data(did, ProfileRepositoryKind::Local).await {
        Ok(data) => {
            debug!("Fetched data for profile {}", did_path);
            HttpResponse::Ok().json(data)
        }
        Err(e) => {
            error!("Failed to fetch profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn set_avatar(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    avatar: web::Json<DataUri>,
) -> HttpResponse {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match set_avatar_in_state(&mut state.vault, &did_path, &avatar).await {
        Ok(()) => {
            debug!("Set profile {} avatar", &did_path);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to set avatar: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

pub async fn set_did_attribute(
    state: web::Data<Mutex<DaemonState>>,
    attr_path: web::Path<AttributePath>,
    attr_val: web::Json<String>,
) -> HttpResponse {
    let did = match did_opt(&attr_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.set_attribute(did, &attr_path.attribute_id, &attr_val).await {
        Ok(()) => {
            debug!("Set attribute {:?} to {}", &attr_path, &attr_val);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to set attribute: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn clear_did_attribute(
    state: web::Data<Mutex<DaemonState>>,
    attr_path: web::Path<AttributePath>,
) -> HttpResponse {
    let did = match did_opt(&attr_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.clear_attribute(did, &attr_path.attribute_id).await {
        Ok(()) => {
            debug!("Cleared attribute {:?}", attr_path);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to clear attribute: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn sign_claim(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    claim: String,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let claim = match claim.parse::<SignableClaimPart>() {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(claim) => claim,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.vault.sign_claim(did, &claim).await {
        Ok(proof) => {
            debug!("Signed claim with profile {}", &did_path);
            HttpResponse::Ok().body(proof.to_string())
        }
        Err(e) => {
            error!("Signing claim failed: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn list_did_claims(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_did_claims_impl(&state.vault, did).await {
        Ok(did_claims) => {
            debug!("Fetched list of claims for did {}", &did_path);
            HttpResponse::Ok().json(did_claims)
        }
        Err(e) => {
            error!("Failed to fetch list of claims: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn list_vault_claims(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_vault_claims_from_state(&state.vault).await {
        Ok(claims) => {
            debug!("Fetched list of claims");
            HttpResponse::Ok().json(claims)
        }
        Err(e) => {
            error!("Failed to fetch list of claims: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn create_did_claim(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    claim_details: web::Json<CreateClaim>,
) -> HttpResponse {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    let did = match did_res(&state.vault, &did_path).await {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };

    let claim = Claim::unproven(
        did.clone(),
        claim_details.schema.to_owned(),
        claim_details.content.to_owned(),
    );
    let claim_id = claim.id();
    match state.vault.add_claim(Some(did), claim).await {
        Ok(()) => {
            debug!("Created claim for did {}", &did_path);
            HttpResponse::Created().json(claim_id)
        }
        Err(e) => {
            debug!("Failed to create claim for did {}: {}", &did_path, e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn delete_claim(
    state: web::Data<Mutex<DaemonState>>,
    claim_path: web::Path<ClaimPath>,
) -> HttpResponse {
    let did = match did_opt(&claim_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.vault.remove_claim(did, claim_path.claim_id.to_owned()).await {
        Ok(()) => {
            debug!("Deleted claim {:?}", &claim_path);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to delete claim {:?}: {}", &claim_path, e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn request_claim_signature(
    state: web::Data<Mutex<DaemonState>>,
    claim_path: web::Path<ClaimPath>,
) -> HttpResponse {
    let did = match did_opt(&claim_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    let claim_path_clone = claim_path.to_owned();
    let profile_claim_fut = async move {
        let profile =
            state.vault.get_profile_data(did.clone(), ProfileRepositoryKind::Local).await?;
        let profile_claim = profile
            .claim(&claim_path.claim_id)
            .map(|claim| claim.signable_part().to_string())
            .ok_or_else(|| format_err!("Claim {:?} not found", &claim_path))?;
        Fallible::Ok(profile_claim)
    };

    match profile_claim_fut.await {
        Ok(signable) => {
            debug!("Created claim signature request for {:?}", &claim_path_clone);
            HttpResponse::Ok().body(signable)
        }
        Err(e) => {
            error!("Failed to serve claim signature request: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

pub async fn add_claim_proof(
    state: web::Data<Mutex<DaemonState>>,
    claim_path: web::Path<ClaimPath>,
    proof: String,
) -> HttpResponse {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match add_claim_proof_to_state(&mut state.vault, &claim_path, &proof).await {
        Ok(()) => {
            debug!("Claim witness signature added");
            HttpResponse::Created().body("")
        }
        Err(e) => {
            error!("Failed to add witness signature: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

pub async fn list_schemas(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.vault.claim_schemas().await {
        Ok(schemas) => {
            debug!("Fetched list of claim schemas");
            let schemas = schemas.iter().map(|v| v.into()).collect::<Vec<ClaimSchema>>();
            HttpResponse::Ok().json(schemas)
        }
        Err(e) => {
            error!("Failed to fetch list of claim schemas: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn list_homes(state: web::Data<Mutex<DaemonState>>) -> HttpResponse {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.network.homes() {
        Ok(homes) => {
            debug!("Fetched list of home nodes");
            HttpResponse::Ok().json(homes)
        }
        Err(e) => {
            error!("Failed to fetch list of home nodes: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn list_did_homes(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.vault.did_homes(did).await {
        Ok(homes) => {
            debug!("Fetched list of home nodes");
            HttpResponse::Ok().json(homes)
        }
        Err(e) => {
            error!("Failed to fetch list of home nodes: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn register_did_home(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
    reg_data: web::Json<HomeRegistration>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state_guard = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    let home_id: ProfileId = match reg_data.home_did.parse() {
        Ok(id) => id,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let addr_hints: Vec<Multiaddr> = match &reg_data.addr_hints {
        None => vec![],
        Some(v) => v.iter().filter_map(|s| s.parse().ok()).collect(),
    };

    let state = &mut *state_guard;
    match state.vault
        .register_home(did.clone(), &home_id, &addr_hints, &mut state.network)
        .await
    {
        Ok(homes) => {
            debug!("Registered new home {} to profile {:?}", reg_data.home_did, did);
            HttpResponse::Created().json(homes)
        }
        Err(e) => {
            error!("Failed to fetch list of home nodes: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub async fn leave_did_home(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    unimplemented!()
}

pub async fn set_did_home_online(
    state: web::Data<Mutex<DaemonState>>,
    did_path: web::Path<String>,
) -> HttpResponse {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    unimplemented!()
}

}

fn lock_state(state: &Mutex<DaemonState>) -> Fallible<MutexGuard<DaemonState>> {
    state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))
}

fn did_opt(did_str: &str) -> Fallible<Option<ProfileId>> {
    if did_str == "_" {
        return Ok(None);
    }
    let did = did_str.parse()?;
    Ok(Some(did))
}

async fn did_res(state: &VaultState, did_str: &str) -> Fallible<ProfileId> {
    did_opt(did_str)?
        .or(state.get_active_profile().await?)
        .ok_or_else(|| err_msg("No profile specified and no active profile set in vault"))
}

// TODO consider moving this to the state
async fn reset_label_if_empty(
    state: &mut VaultState,
    label: &mut String,
    did: &ProfileId,
) -> Fallible<()> {
    if label.trim().is_empty() {
        let hd_label = DeterministicNameGenerator::default().name(&did.to_bytes());
        state.set_profile_label(Some(did.clone()), hd_label.clone()).await?;
        *label = hd_label;
    }
    Ok(())
}

async fn create_dids_to_state(state: &mut VaultState, label: &mut String) -> Fallible<VaultEntry> {
    debug!("Creating profile with label '{}'", label);
    let profile = state.create_profile(Some(label.clone())).await?;
    let did = profile.id();
    let did_bytes = did.to_bytes();

    reset_label_if_empty(state, label, &did).await?;

    let mut avatar_png = Vec::new();
    blockies::Ethereum::default()
        .create_icon(&mut avatar_png, &did_bytes)
        .map_err(|e| err_msg(format!("Failed to generate default profile icon: {:?}", e)))?;
    //std::fs::write(format!("/tmp/{}.png", label), &avatar_png)?;

    let mut metadata = PersonaCustomData::default();
    metadata.image_blob = avatar_png;
    metadata.image_format = "png".to_owned();
    state.set_profile_metadata(Some(did.clone()), metadata.clone().try_into()?).await?;

    state.save_vault()?;
    Ok(VaultEntry {
        id: did.to_string(),
        label: label.to_owned(),
        avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
        state: "TODO".to_owned(),
    })
}

async fn set_avatar_in_state(
    state: &mut VaultState,
    did_str: &String,
    avatar_datauri: &DataUri,
) -> Fallible<()> {
    let did = did_opt(did_str)?;
    let (format, avatar_binary) = parse_avatar(&avatar_datauri)?;
    let metadata_ser = state.get_profile_metadata(did.clone()).await?;
    let mut metadata = PersonaCustomData::try_from(metadata_ser.as_str())?;
    metadata.image_format = format;
    metadata.image_blob = avatar_binary;
    state.set_profile_metadata(did, metadata.try_into()?).await
}

// TODO consider changing state operation return types to ApiClaim
async fn list_did_claims_impl(
    state: &VaultState,
    did: Option<ProfileId>,
) -> Fallible<Vec<ApiClaim>> {
    let claims = state.claims(did.clone()).await?;
    let rec = state.get_vault_record(did).await?;
    let schema_registry = state.claim_schemas().await?;
    let claims = claims
        .iter()
        .filter_map(|claim| {
            let res = ApiClaim::try_from(claim, rec.label(), &*schema_registry);
            if res.is_err() {
                error!("Failed to convert claim {:?} for HTTP API: {:?}", claim, res);
            }
            res.ok()
        })
        .collect();
    Ok(claims)
}

async fn list_vault_claims_from_state(state: &VaultState) -> Fallible<Vec<ApiClaim>> {
    let schema_registry = state.claim_schemas().await?;

    let mut claims = Vec::new();
    for rec in state.list_vault_records().await? {
        let did_claims = state.claims(Some(rec.id())).await?;
        for claim in did_claims {
            claims.push(ApiClaim::try_from(&claim, rec.label(), &*schema_registry)?);
        }
    }

    Ok(claims)
}

async fn list_dids_from_state(state: &VaultState) -> Fallible<Vec<VaultEntry>> {
    let recs = state.list_vault_records().await?;
    let entries = recs
        .iter()
        .filter_map(|record| {
            let res = VaultEntry::try_from(record);
            if res.is_err() {
                error!("Failed to convert vault record {:?} for HTTP API: {:?}", record, res);
            }
            res.ok()
        })
        .collect();
    Ok(entries)
}

async fn add_claim_proof_to_state(
    state: &mut VaultState,
    claim_path: &ClaimPath,
    proof_str: &String,
) -> Fallible<()> {
    let did = did_opt(&claim_path.did)?;
    let claim_id = &claim_path.claim_id;
    let proof: ClaimProof = proof_str.parse()?;
    let profile = state.get_profile_data(did.clone(), ProfileRepositoryKind::Local).await?;
    let claim = profile
        .claim(claim_id)
        .ok_or_else(|| format_err!("Claim {} not found in profile {:?}", claim_id, did))?;
    proof.validate(claim.signable_part())?;
    state.add_claim_proof(did, claim_id, proof).await
}
