use std::convert::{TryFrom, TryInto};
use std::sync::{Mutex, MutexGuard};

use actix_web::{web, HttpResponse, Responder};
use failure::{err_msg, format_err, Fallible};
use log::*;

use crate::names::DeterministicNameGenerator;
use crate::vault::api_impl::VaultApiImpl;
use crate::*;
use claims::model::*;
use keyvault::Seed;

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

// TODO this Fallible -> Responder mapping + logging should be less manual,
//      at least parts should be generated, e.g. using macros
pub fn init_vault(
    state: web::Data<Mutex<VaultApiImpl>>,
    words: web::Json<Vec<String>>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    let phrase = words.join(" ");
    match state.restore_vault(phrase) {
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

pub fn restore_all_dids(state: web::Data<Mutex<VaultApiImpl>>) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.restore_all_profiles() {
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

pub fn get_default_did(state: web::Data<Mutex<VaultApiImpl>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.get_active_profile() {
        Ok(did_opt) => HttpResponse::Ok().json(did_opt.map(|did| did.to_string())),
        Err(e) => {
            error!("Failed to get default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn set_default_did(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_str: web::Json<String>,
) -> impl Responder {
    let did = match did_str.parse::<ProfileId>() {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.set_active_profile(&did) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => {
            error!("Failed to set default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_did(state: web::Data<Mutex<VaultApiImpl>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_dids_from_state(&state) {
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

pub fn list_dids_from_state(state: &VaultApiImpl) -> Fallible<Vec<VaultEntry>> {
    let recs = state.list_vault_records()?;
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

pub fn create_did(
    state: web::Data<Mutex<VaultApiImpl>>,
    mut label: web::Json<ProfileLabel>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match create_dids_to_state(&mut state, &mut label) {
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

pub fn create_dids_to_state(state: &mut VaultApiImpl, label: &mut String) -> Fallible<VaultEntry> {
    debug!("Creating profile with label '{}'", label);
    let profile = state.create_profile(Some(label.clone()))?;
    let did = profile.id();
    let did_bytes = did.to_bytes();

    reset_label_if_empty(state, label, &did)?;

    let mut avatar_png = Vec::new();
    blockies::Ethereum::default()
        .create_icon(&mut avatar_png, &did_bytes)
        .map_err(|e| err_msg(format!("Failed to generate default profile icon: {:?}", e)))?;
    //std::fs::write(format!("/tmp/{}.png", label), &avatar_png)?;

    let mut metadata = PersonaCustomData::default();
    metadata.image_blob = avatar_png;
    metadata.image_format = "png".to_owned();
    state.set_profile_metadata(Some(did.clone()), metadata.clone().try_into()?)?;

    state.save_vault()?;
    Ok(VaultEntry {
        id: did.to_string(),
        label: label.to_owned(),
        avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
        state: "TODO".to_owned(),
    })
}

pub fn get_did(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.get_vault_record(did).and_then(|rec| VaultEntry::try_from(&rec)) {
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

pub fn restore(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    force: web::Json<bool>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.restore_profile(did, *force) {
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

pub fn revert(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.revert_profile(did) {
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

pub fn publish(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    force: web::Json<bool>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.publish_profile(did, *force) {
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

pub fn rename_did(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    mut label: web::Json<ProfileLabel>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    let did = match did_res(&state, &did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    match reset_label_if_empty(&mut state, &mut label, &did)
        .and_then(|()| state.set_profile_label(Some(did), label.to_owned()))
    {
        Ok(()) => {
            debug!("Renamed profile {} to {}", &did_path, label);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to rename profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn get_profile(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.get_profile_data(did, ProfileRepositoryKind::Local) {
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

pub fn set_avatar(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    avatar: web::Json<DataUri>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match set_avatar_in_state(&mut state, &did_path, &avatar) {
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

fn set_avatar_in_state(
    state: &mut VaultApiImpl,
    did_str: &String,
    avatar_datauri: &DataUri,
) -> Fallible<()> {
    let did = did_opt(did_str)?;
    let (format, avatar_binary) = parse_avatar(&avatar_datauri)?;
    let metadata_ser = state.get_profile_metadata(did.clone())?;
    let mut metadata = PersonaCustomData::try_from(metadata_ser.as_str())?;
    metadata.image_format = format;
    metadata.image_blob = avatar_binary;
    state.set_profile_metadata(did, metadata.try_into()?)
}

pub fn set_did_attribute(
    state: web::Data<Mutex<VaultApiImpl>>,
    attr_path: web::Path<AttributePath>,
    attr_val: web::Json<String>,
) -> impl Responder {
    let did = match did_opt(&attr_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.set_attribute(did, &attr_path.attribute_id, &attr_val) {
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

pub fn clear_did_attribute(
    state: web::Data<Mutex<VaultApiImpl>>,
    attr_path: web::Path<AttributePath>,
) -> impl Responder {
    let did = match did_opt(&attr_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match state.clear_attribute(did, &attr_path.attribute_id) {
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

pub fn sign_claim(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    claim: String,
) -> impl Responder {
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
    match state.sign_claim(did, &claim) {
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

pub fn list_did_claims(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
) -> impl Responder {
    let did = match did_opt(&did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_did_claims_impl(&state, did) {
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

// TODO consider changing state operation return types to ApiClaim
fn list_did_claims_impl(state: &VaultApiImpl, did: Option<ProfileId>) -> Fallible<Vec<ApiClaim>> {
    let claims = state.claims(did.clone())?;
    let rec = state.get_vault_record(did)?;
    let schema_registry = state.claim_schemas()?;
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

pub fn list_vault_claims(state: web::Data<Mutex<VaultApiImpl>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_vault_claims_from_state(&state) {
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

fn list_vault_claims_from_state(state: &VaultApiImpl) -> Fallible<Vec<ApiClaim>> {
    let schema_registry = state.claim_schemas()?;

    let mut claims = Vec::new();
    for rec in state.list_vault_records()? {
        let did_claims = state.claims(Some(rec.id()))?;
        for claim in did_claims {
            claims.push(ApiClaim::try_from(&claim, rec.label(), &*schema_registry)?);
        }
    }

    Ok(claims)
}

pub fn create_did_claim(
    state: web::Data<Mutex<VaultApiImpl>>,
    did_path: web::Path<String>,
    claim_details: web::Json<CreateClaim>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    let did = match did_res(&state, &did_path) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };

    let claim = Claim::unproven(
        did.clone(),
        claim_details.schema.to_owned(),
        claim_details.content.to_owned(),
    );
    let claim_id = claim.id();
    match state.add_claim(Some(did), claim) {
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

pub fn delete_claim(
    state: web::Data<Mutex<VaultApiImpl>>,
    claim_path: web::Path<ClaimPath>,
) -> impl Responder {
    let did = match did_opt(&claim_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.remove_claim(did, claim_path.claim_id.to_owned()) {
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

pub fn request_claim_signature(
    state: web::Data<Mutex<VaultApiImpl>>,
    claim_path: web::Path<ClaimPath>,
) -> impl Responder {
    let did = match did_opt(&claim_path.did) {
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
        Ok(did) => did,
    };
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    let profile_claim =
        state.get_profile_data(did.clone(), ProfileRepositoryKind::Local).and_then(|profile| {
            profile
                .claim(&claim_path.claim_id)
                .map(|claim| claim.signable_part().to_string())
                .ok_or_else(|| format_err!("Claim {:?} not found", &claim_path))
        });

    match profile_claim {
        Ok(signable) => {
            debug!("Created claim signature request for {:?}", &claim_path);
            HttpResponse::Ok().body(signable)
        }
        Err(e) => {
            error!("Failed to serve claim signature request: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

pub fn add_claim_proof(
    state: web::Data<Mutex<VaultApiImpl>>,
    claim_path: web::Path<ClaimPath>,
    proof: String,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match add_claim_proof_to_state(&mut state, &claim_path, &proof) {
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

pub fn add_claim_proof_to_state(
    state: &mut VaultApiImpl,
    claim_path: &ClaimPath,
    proof_str: &String,
) -> Fallible<()> {
    let did = did_opt(&claim_path.did)?;
    let claim_id = &claim_path.claim_id;
    let proof: ClaimProof = proof_str.parse()?;
    let profile = state.get_profile_data(did.clone(), ProfileRepositoryKind::Local)?;
    let claim = profile
        .claim(claim_id)
        .ok_or_else(|| format_err!("Claim {} not found in profile {:?}", claim_id, did))?;
    proof.validate(claim.signable_part())?;
    state.add_claim_proof(did, claim_id, proof)
}

pub fn list_schemas(state: web::Data<Mutex<VaultApiImpl>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };

    match state.claim_schemas() {
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

fn did_opt(did_str: &str) -> Fallible<Option<ProfileId>> {
    if did_str == "_" {
        return Ok(None);
    }
    let did = did_str.parse()?;
    Ok(Some(did))
}

fn did_res(state: &VaultApiImpl, did_str: &str) -> Fallible<ProfileId> {
    did_opt(did_str)?
        .or(state.get_active_profile()?)
        .ok_or_else(|| err_msg("No profile specified and no active profile set in vault"))
}

fn lock_state(state: &web::Data<Mutex<VaultApiImpl>>) -> Fallible<MutexGuard<VaultApiImpl>> {
    state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))
}

// TODO consider moving this to the state
fn reset_label_if_empty(
    state: &mut VaultApiImpl,
    label: &mut String,
    did: &ProfileId,
) -> Fallible<()> {
    if label.is_empty() || label.find(|c| !char::is_whitespace(c)).is_none() {
        let hd_label = DeterministicNameGenerator::default().name(&did.to_bytes());
        state.set_profile_label(Some(did.clone()), hd_label.clone())?;
        *label = hd_label;
    }
    Ok(())
}
