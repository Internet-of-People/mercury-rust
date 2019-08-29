use std::sync::Mutex;

use actix_web::{web, HttpResponse, Responder};
use log::*;

use crate::data::{ClaimPath, CreateClaim, DataUri};
use crate::imp::*;
use claims::api::*;
use did::vault::*;

// TODO make URLs const variables and share them between server and client
pub fn init_url_mapping(service: &mut web::ServiceConfig) {
    service
        .service(
            web::scope("/bip39")
                .service(web::resource("").route(web::post().to(generate_bip39_phrase)))
                .service(
                    web::resource("/validate-phrase").route(web::post().to(validate_bip39_phrase)),
                )
                .service(
                    web::resource("/validate-word").route(web::post().to(validate_bip39_word)),
                ),
        )
        .service(
            web::scope("/vault")
                .service(web::resource("").route(web::post().to(init_vault)))
                .service(
                    web::scope("/dids")
                        .service(
                            web::resource("")
                                .route(web::get().to(list_did))
                                .route(web::post().to(create_did)),
                        )
                        .service(
                            web::scope("/{did}")
                                .service(web::resource("").route(web::get().to(get_did)))
                                .service(web::resource("/label").route(web::put().to(rename_did)))
                                .service(web::resource("/avatar").route(web::put().to(set_avatar)))
                                .service(
                                    web::scope("/claims")
                                        .service(
                                            web::resource("")
                                                .route(web::get().to(list_did_claims))
                                                .route(web::post().to(create_claim)),
                                        )
                                        .service(
                                            web::resource("{claim_id}")
                                                .route(web::delete().to(delete_claim)),
                                        ),
                                ),
                        ),
                )
                .service(web::resource("/claims").route(web::get().to(list_vault_claims))),
        )
        .service(web::resource("/claim-schemas").route(web::get().to(list_schemas)));
}

// TODO this Fallible -> Responder mapping + logging should be somehow less manual
fn init_vault(state: web::Data<Mutex<Context>>, words: web::Json<Vec<String>>) -> impl Responder {
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

fn list_did(state: web::Data<Mutex<Context>>) -> impl Responder {
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

fn create_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    match create_dids_impl(state) {
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

fn get_did(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
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

fn rename_did(
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

fn set_avatar(
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

fn list_did_claims(state: web::Data<Mutex<Context>>, did: web::Path<String>) -> impl Responder {
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

fn list_vault_claims(state: web::Data<Mutex<Context>>) -> impl Responder {
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

fn create_claim(
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

fn delete_claim(
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

fn list_schemas(state: web::Data<Mutex<Context>>) -> impl Responder {
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
