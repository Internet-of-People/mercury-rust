use std::sync::{Mutex, MutexGuard};

use actix_web::{web, HttpResponse, Responder};
use failure::{err_msg, Fallible};
use log::*;

use crate::imp::*;
use crate::*;
use claims::model::*;
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

pub fn lock_state(state: &web::Data<Mutex<Context>>) -> Fallible<MutexGuard<Context>> {
    state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))
}

// TODO this Fallible -> Responder mapping + logging should be less manual,
//      at least parts should be generated, e.g. using macros
pub fn init_vault(
    state: web::Data<Mutex<Context>>,
    words: web::Json<Vec<String>>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match init_vault_impl(&mut state, &words) {
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
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match restore_all_dids_impl(&mut state) {
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
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match get_default_did_impl(&state) {
        Ok(did_opt) => HttpResponse::Ok().json(did_opt.map(|did| did.to_string())),
        Err(e) => {
            error!("Failed to get default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn set_default_did(state: web::Data<Mutex<Context>>, did: web::Json<String>) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match set_default_did_impl(&mut state, &did) {
        Ok(()) => HttpResponse::Ok().body(""),
        Err(e) => {
            error!("Failed to set default profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_dids_impl(&state) {
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

pub fn restore(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    force: web::Json<bool>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match restore_did_impl(&mut state, &did, *force) {
        Ok(priv_data) => {
            debug!("Restored profile {}", did);
            HttpResponse::Ok().json(priv_data) // TODO consider security here
        }
        Err(e) => {
            error!("Failed to restore profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn revert(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match revert_did_impl(&mut state, &did) {
        Ok(priv_data) => {
            debug!("Reverted profile {}", did);
            HttpResponse::Ok().json(priv_data) // TODO consider security here
        }
        Err(e) => {
            error!("Failed to revert profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn publish(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    force: web::Json<bool>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match publish_did_impl(&mut state, &did, *force) {
        Ok(id) => {
            debug!("Published profile {}", did);
            HttpResponse::Ok().json(id)
        }
        Err(e) => {
            error!("Failed to publish profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn create_did(
    state: web::Data<Mutex<Context>>,
    mut label: web::Json<ProfileLabel>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match create_dids_impl(&mut state, &mut label) {
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
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match get_did_impl(&state, &did) {
        Ok(entry) => {
            debug!("Fetched info for profile {}", did);
            HttpResponse::Ok().json(entry)
        }
        Err(e) => {
            error!("Failed to get profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn rename_did(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    //did: web::Path<ProfileId>,
    mut label: web::Json<ProfileLabel>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match rename_did_impl(&mut state, &did, &mut label) {
        Ok(()) => {
            debug!("Renamed profile {} to {}", did, label);
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
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match set_avatar_impl(&mut state, &did, &avatar) {
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

// TODO this directly exposes Private/PublicProfileData structs
// TODO exposing private data should be secured
pub fn get_profile(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match get_profile_impl(&state, &did) {
        Ok(data) => {
            debug!("Fetched data for profile {}", did);
            HttpResponse::Ok().json(data)
        }
        Err(e) => {
            error!("Failed to fetch profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn set_did_attribute(
    state: web::Data<Mutex<Context>>,
    attr_path: web::Path<AttributePath>,
    attr_val: web::Json<String>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match set_did_attribute_impl(&mut state, &attr_path, &attr_val) {
        Ok(()) => {
            debug!("Set attribute {:?} to {}", attr_path, attr_val);
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to set attribute: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn clear_did_attribute(
    state: web::Data<Mutex<Context>>,
    attr_path: web::Path<AttributePath>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match clear_did_attribute_impl(&mut state, &attr_path) {
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
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    claim: web::Json<SignableClaimPart>,
) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match sign_claim_impl(&state, &did, &claim) {
        Ok(signed_claim) => {
            debug!("Signed claim with profile {}", did);
            HttpResponse::Ok().json(signed_claim)
        }
        Err(e) => {
            error!("Signing claim failed: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_did_claims(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_did_claims_impl(&state, &did) {
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
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_vault_claims_impl(&state) {
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

pub fn create_did_claim(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    claim_details: web::Json<CreateClaim>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match create_did_claim_impl(&mut state, &did, &claim_details) {
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
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match delete_claim_impl(&mut state, &claim_path) {
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

pub fn request_claim_signature(
    state: web::Data<Mutex<Context>>,
    claim_path: web::Path<ClaimPath>,
) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match request_claim_signature_impl(&state, &claim_path) {
        Ok(message) => {
            debug!("Created claim signature request");
            HttpResponse::Ok().json(message)
        }
        Err(e) => {
            error!("Failed to serve claim signature request: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn add_claim_proof(
    state: web::Data<Mutex<Context>>,
    claim_path: web::Path<ClaimPath>,
    // TODO consider if exposing SignedMessage gusiness logic type here is a good choice
    proof: web::Json<ClaimProof>,
) -> impl Responder {
    let mut state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match add_claim_proof_impl(&mut state, &claim_path, &proof) {
        Ok(()) => {
            debug!("Claim witness signature added");
            HttpResponse::Created().body("")
        }
        Err(e) => {
            error!("Failed to add witness signature: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

pub fn list_schemas(state: web::Data<Mutex<Context>>) -> impl Responder {
    let state = match lock_state(&state) {
        Err(e) => return HttpResponse::Conflict().body(e.to_string()),
        Ok(state) => state,
    };
    match list_schemas_impl(&state) {
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
