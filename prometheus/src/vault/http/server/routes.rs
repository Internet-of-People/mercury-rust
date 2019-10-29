use actix_web::web;
//use log::*;

use super::controller::*;

// TODO make URLs const variables and share them between server and client
// or create a structopt-like tree-structure to have a type-safe representation of our API
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
                .service(web::resource("").route(web::post().to_async(init_vault)))
                .service(web::resource("/restore-dids").route(web::post().to_async(restore_all_dids)))
                .service(
                    web::scope("/default-did").service(
                        web::resource("")
                            .route(web::get().to_async(get_default_did))
                            .route(web::put().to_async(set_default_did)),
                    ),
                )
                .service(
                    web::scope("/dids")
                        .service(
                            web::resource("")
                                .route(web::get().to_async(list_did))
                                .route(web::post().to_async(create_did)),
                        )
                        .service(
                            web::scope("/{did}")
                                .service(web::resource("").route(web::get().to_async(get_did)))
                                .service(web::resource("/label").route(web::put().to_async(rename_did)))
                                .service(web::resource("/avatar").route(web::put().to_async(set_avatar)))

                                // TODO URL mapping might be misleading here because these calls
                                //      work with a ProfileRepository, not the DidVault itself.
                                //      Consider a different mapping, e.g. /profiles/{did}/...
                                .service(web::resource("/profiledata").route(web::get().to_async(get_profile)))
                                .service(web::resource("/restore").route(web::post().to_async(restore)))
                                .service(web::resource("/revert").route(web::post().to_async(revert)))
                                .service(web::resource("/publish").route(web::post().to_async(publish)))
                                .service(
                                web::resource("/attributes/{attribute_id}")
                                    .route(web::post().to_async(set_did_attribute))
                                    .route(web::delete().to_async(clear_did_attribute)),
                                )
                                .service( web::resource("/sign-claim").route(web::post().to_async(sign_claim)))
                                .service(
                                    web::scope("/claims")
                                        .service(
                                            web::resource("")
                                                .route(web::get().to_async(list_did_claims))
                                                .route(web::post().to_async(create_did_claim)),
                                        )
                                        .service(
                                            web::scope("{claim_id}")
                                                .service(web::resource("")
                                                    .route(web::delete().to_async(delete_claim)),
                                                )
                                                .service(web::resource("/witness-request")
                                                    .route(web::get().to_async(request_claim_signature))
                                                )
                                                .service(web::resource("/witness-signature")
                                                    .route(web::put().to_async(add_claim_proof))
                                                )
                                        ),
                                )
                                .service(web::scope("/homes")
                                    .service(web::resource("")
                                        .route(web::get().to_async(list_did_homes))
                                        .route(web::post().to_async(register_did_home))
                                    )
                                    .service(web::scope("{home_did}")
                                        .service(web::resource("")
                                            .route(web::delete().to_async(leave_did_home))
                                        )
                                        .service(web::resource("online")
                                            .route(web::put().to_async(set_did_home_online))
                                        )
                                    )
                                ),
                        ),
                )
                .service(web::resource("/claims").route(web::get().to_async(list_vault_claims))),
        )
        .service(web::resource("/homes").route(web::get().to_async(list_homes)))
        .service(web::resource("/claim-schemas").route(web::get().to_async(list_schemas)));
}
