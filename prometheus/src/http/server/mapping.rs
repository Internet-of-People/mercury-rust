use actix_web::{web, HttpResponse, Responder};
use log::*;

use crate::data::{ClaimPath, CreateClaim, DataUri};
use crate::http::server::status::*;
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
                .service(web::resource("/restore-dids").route(web::post().to(restore_all_dids)))
                .service(
                    web::scope("/default-did").service(
                        web::resource("")
                            .route(web::get().to(get_default_did))
                            .route(web::put().to(set_default_did)),
                    ),
                )
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
