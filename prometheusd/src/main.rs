use actix_web::{web, App, HttpResponse, HttpServer, Responder};

use keyvault::Seed;

fn generate_bip39_phrase() -> impl Responder {
    Seed::generate_bip39()
}

fn validate_bip39_phrase(phrase: web::Path<String>) -> impl Responder {
    match Seed::from_bip39(&phrase as &str) {
        Ok(_seed) => HttpResponse::Accepted().body("Valid"),
        Err(e) => HttpResponse::NotAcceptable().body(format!("Invalid: {}", e)),
    }
}

fn validate_bip39_word(word: web::Path<String>) -> impl Responder {
    let is_valid = Seed::check_word(&word);
    let mut resp = if is_valid { HttpResponse::Accepted() } else { HttpResponse::NotAcceptable() };
    resp.body(is_valid.to_string())
}

fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            web::scope("/vault")
                .service(web::resource("/generate_phrase").to(generate_bip39_phrase))
                .service(web::resource("/validate_phrase/{phrase}").to(validate_bip39_phrase))
                .service(web::resource("/validate_word/{word}").to(validate_bip39_word)),
        )
    })
    .bind("127.0.0.1:8080")?
    .run()
}
