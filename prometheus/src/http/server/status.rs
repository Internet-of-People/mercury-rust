use std::sync::Mutex;

use actix_web::{web, HttpResponse, Responder};
use log::*;

use crate::data::{ClaimPath, CreateClaim, DataUri};
use crate::http::server::imp::*;
use claims::api::*;
use did::vault::*;
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

// TODO this Fallible -> Responder mapping + logging should be somehow less manual
pub fn init_vault(
    state: web::Data<Mutex<Context>>,
    words: web::Json<Vec<String>>,
) -> impl Responder {
    match init_vault_impl(state, words) {
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

pub fn restore_all_dids(state: web::Data<Mutex<Context>>) -> impl Responder {
    match restore_all_dids_impl(state) {
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

pub fn get_default_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    match get_default_did_impl(state) {
        Ok(did_opt) => HttpResponse::Ok().json(did_opt.map(|did| did.to_string())),
        Err(e) => {
            error!("Failed to get default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn set_default_did(state: web::Data<Mutex<Context>>, did: web::Json<String>) -> impl Responder {
    info!("Setting default did: {:?}", did);
    match set_default_did_impl(state, did.clone()) {
        Ok(entry) => HttpResponse::Ok().json(entry),
        Err(e) => {
            error!("Failed to set default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    match list_dids_impl(state) {
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

pub fn create_did(
    state: web::Data<Mutex<Context>>,
    label: web::Json<Option<ProfileLabel>>,
) -> impl Responder {
    match create_dids_impl(state, label.clone()) {
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

pub fn get_did(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
    match get_did_impl(state, did.clone()) {
        Ok(entry) => {
            debug!("Fetched info for profile {}", did);
            HttpResponse::Ok().json(entry)
        }
        Err(e) => {
            error!("Failed to create profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn rename_did(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    //did: web::Path<ProfileId>,
    name: web::Json<ProfileLabel>,
) -> impl Responder {
    match rename_did_impl(state, did.clone(), name.clone()) {
        Ok(()) => {
            debug!("Renamed profile {} to {}", did, name);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to rename profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn set_avatar(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    avatar: web::Json<DataUri>,
) -> impl Responder {
    match set_avatar_impl(state, did.clone(), avatar.clone()) {
        Ok(()) => {
            debug!("Set profile {} avatar", did);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to set avatar: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_did_claims(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
    match list_did_claims_impl(state, did.clone()) {
        Ok(did_claims) => {
            debug!("Fetched list of claims for did {}", did);
            HttpResponse::Ok().json(did_claims)
        }
        Err(e) => {
            error!("Failed to fetch list of claims: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_vault_claims(state: web::Data<Mutex<Context>>) -> impl Responder {
    match list_vault_claims_impl(state) {
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

pub fn create_claim(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    claim_details: web::Json<CreateClaim>,
) -> impl Responder {
    match create_claim_impl(state, did.clone(), claim_details.clone()) {
        Ok(claim) => {
            debug!("Created claim for did {}", did);
            HttpResponse::Created().json(claim)
        }
        Err(e) => {
            debug!("Failed to create claim for did {}: {}", did, e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn delete_claim(
    state: web::Data<Mutex<Context>>,
    claim_path: web::Path<ClaimPath>,
) -> impl Responder {
    match delete_claim_impl(state, claim_path.clone()) {
        Ok(()) => {
            debug!("Deleted claim {} from profile {}", claim_path.claim_id, claim_path.did);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!(
                "Failed to delete claim {} from profile {}: {}",
                claim_path.claim_id, claim_path.did, e
            );
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_schemas(state: web::Data<Mutex<Context>>) -> impl Responder {
    match list_schemas_impl(state) {
        Ok(list) => {
            debug!("Fetched list of claim schemas");
            HttpResponse::Ok().json(list)
        }
        Err(e) => {
            error!("Failed to fetch list of claim schemas: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}
