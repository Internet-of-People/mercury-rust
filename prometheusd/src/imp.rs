use std::convert::{TryFrom, TryInto};
use std::sync::{Mutex, MutexGuard};

use actix_web::{web, HttpResponse, Responder};
use failure::{bail, err_msg, Fallible};

use crate::data::{ProfileMetadata, *};
use claims::{api::*, model::*};
use did::vault::ProfileAlias;
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

fn lock_state(state: &web::Data<Mutex<Context>>) -> Fallible<MutexGuard<Context>> {
    state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))
}

pub fn init_vault_impl(
    state: web::Data<Mutex<Context>>,
    words: web::Json<Vec<String>>,
) -> Fallible<()> {
    let mut state = lock_state(&state)?;
    let phrase = words.join(" ");
    state.restore_vault(phrase)?;
    state.save_vault()
}

pub fn list_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<VaultEntry>> {
    let state = lock_state(&state)?;
    let recs = state.list_vault_records()?;
    // TODO we should also log errors here if any occurs during the conversion
    let entries = recs.iter().filter_map(|rec| VaultEntry::try_from(rec).ok()).collect::<Vec<_>>();
    Ok(entries)
}

pub fn create_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<VaultEntry> {
    let mut state = lock_state(&state)?;

    //let alias = state.list_profiles()?.len().to_string();
    // TODO this name generation is not deterministic, but should be (found no proper lib)
    // TODO instantiating generators here might provide worse performance than keeping
    //      a generator instance in the state, but that is probably not significant in practice
    let alias = names::Generator::default().next().unwrap_or_else(|| "FAILING FAILURE".to_owned());
    let did = state.create_profile(alias.clone())?;

    let mut avatar_png = Vec::new();
    blockies::Ethereum::default()
        .create_icon(&mut avatar_png, &did.to_bytes())
        .map_err(|e| err_msg(format!("Failed to generate default profile icon: {:?}", e)))?;
    //std::fs::write(format!("/tmp/{}.png", alias), &avatar_png)?;

    let mut metadata = ProfileMetadata::default();
    metadata.image_blob = avatar_png;
    metadata.image_format = "png".to_owned();
    state.set_profile_metadata(Some(did.clone()), metadata.clone().try_into()?)?;

    state.save_vault()?;
    Ok(VaultEntry {
        id: did.to_string(),
        alias,
        avatar: Image { format: metadata.image_format, blob: metadata.image_blob },
        state: "TODO".to_owned(),
    })
}

pub fn get_did_impl(state: web::Data<Mutex<Context>>, did_str: String) -> Fallible<VaultEntry> {
    let did = did_str.parse()?;
    let state = lock_state(&state)?;
    let rec = state.get_vault_record(Some(did))?;
    VaultEntry::try_from(&rec)
}

pub fn rename_did_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
    //did: ProfileId,
    name: ProfileAlias,
) -> Fallible<()> {
    let did = did_str.parse()?;
    let mut state = lock_state(&state)?;
    state.rename_profile(Some(did), name)?;
    state.save_vault()?;
    Ok(())
}

pub fn set_avatar_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
    avatar_datauri: DataUri,
) -> Fallible<()> {
    let did: ProfileId = did_str.parse()?;
    let (format, avatar_binary) = parse_avatar(&avatar_datauri)?;
    let mut state = lock_state(&state)?;
    let metadata_ser = state.get_profile_metadata(Some(did.clone()))?;
    let mut metadata = ProfileMetadata::try_from(metadata_ser.as_str())?;
    metadata.image_format = format;
    metadata.image_blob = avatar_binary;
    state.set_profile_metadata(Some(did), metadata.try_into()?)?;
    state.save_vault()?;
    Ok(())
}

pub fn list_did_claims_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
) -> Fallible<Vec<ApiClaim>> {
    let did = did_str.parse()?;
    let state = lock_state(&state)?;
    let rec = state.get_vault_record(Some(did))?;
    let schema_registry = state.claim_schemas()?;
    let metadata = ProfileMetadata::try_from(rec.metadata().as_str())?;
    let claims = metadata
        .claims
        .iter()
        // TODO at least log conversion errors 
        .filter_map(|claim| ApiClaim::try_from(claim, rec.alias(), &schema_registry).ok())
        .collect();
    Ok(claims)
}

pub fn list_vault_claims_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<ApiClaim>> {
    let state = lock_state(&state)?;
    let schema_registry = state.claim_schemas()?;

    let mut claims = Vec::new();
    for rec in state.list_vault_records()? {
        let metadata = ProfileMetadata::try_from(rec.metadata().as_str())?;
        for claim in metadata.claims {
            claims.push(ApiClaim::try_from(&claim, rec.alias(), &schema_registry)?);
        }
    }

    Ok(claims)
}

pub fn create_claim_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
    claim_details: CreateClaim,
) -> Fallible<ContentId> {
    let did: ProfileId = did_str.parse()?;
    let mut state = lock_state(&state)?;
    let metadata_ser = state.get_profile_metadata(Some(did.clone()))?;
    let mut metadata = ProfileMetadata::try_from(metadata_ser.as_str())?;

    let claim = Claim::new(did.clone(), claim_details.schema, claim_details.content);
    let claim_id = claim.id();

    let conflicting_claims = metadata.claims.iter().filter(|claim| claim.id() == claim_id);
    if conflicting_claims.count() != 0 {
        bail!("Claim {} is already present", claim_id);
    }

    // TODO check if schema_id is valid and related schema contents are available
    // TODO validate contents against schema details
    metadata.claims.push(claim);

    state.set_profile_metadata(Some(did), metadata.try_into()?)?;
    state.save_vault()?;
    Ok(claim_id)
}

pub fn delete_claim_impl(state: web::Data<Mutex<Context>>, claim_path: ClaimPath) -> Fallible<()> {
    let did: ProfileId = claim_path.did.parse()?;
    let claim_id = claim_path.claim_id;
    let mut state = lock_state(&state)?;
    let metadata_ser = state.get_profile_metadata(Some(did.clone()))?;
    let mut metadata = ProfileMetadata::try_from(metadata_ser.as_str())?;

    let claims_len_before = metadata.claims.len();
    metadata.claims.retain(|claim| claim.id() != claim_id);
    if metadata.claims.len() != claims_len_before - 1 {
        bail!("Claim {} not found", claim_id);
    }

    // TODO consider if deleting all related presentations are needed as a separate step

    state.set_profile_metadata(Some(did), metadata.try_into()?)?;
    state.save_vault()?;
    Ok(())
}

pub fn list_schemas_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<ClaimSchema>> {
    let state = lock_state(&state)?;
    let repo = state.claim_schemas()?;
    Ok(repo.iter().map(|v| v.into()).collect::<Vec<_>>())
}
