use actix_web::web;
//use log::*;

use crate::http::server::controller::*;

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

                                // TODO URL mapping might be misleading here because these calls
                                //      work with a ProfileRepository, not the DidVault itself.
                                //      Consider a different mapping, e.g. /profiles/{did}/...
                                .service(web::resource("/profiledata").route(web::get().to(get_profile)))
                                .service(web::resource("/restore").route(web::post().to(restore)))
                                .service(web::resource("/revert").route(web::post().to(revert)))
                                .service(web::resource("/publish").route(web::post().to(publish)))
                                .service(
                                web::resource("/attributes/{attribute_id}")
                                    .route(web::post().to(set_did_attribute))
                                    .route(web::delete().to(clear_did_attribute)),
                                )
                                .service( web::resource("/sign-claim").route(web::post().to(sign_claim)))
                                .service(
                                    web::scope("/claims")
                                        .service(
                                            web::resource("")
                                                .route(web::get().to(list_did_claims))
                                                .route(web::post().to(create_did_claim)),
                                        )
                                        .service(
                                            web::scope("{claim_id}")
                                                .service(web::resource("")
                                                    .route(web::delete().to(delete_claim)),
                                                )
                                                .service(web::resource("/witness-request")
                                                    .route(web::get().to(request_claim_signature))
                                                )
                                                .service(web::resource("/witness-signature")
                                                    .route(web::put().to(add_claim_proof))
                                                )
                                        ),
                                ),
                        ),
                )
                .service(web::resource("/claims").route(web::get().to(list_vault_claims))),
        )
        .service(web::resource("/claim-schemas").route(web::get().to(list_schemas)));
}
