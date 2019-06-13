use actix_web::{web, App, HttpResponse, HttpServer, Responder};

use keyvault::Seed;

fn validate_bip39_word(path: web::Path<(String,)>) -> impl Responder {
    let word = &path.0;
    let is_valid = Seed::check_word(word);
    let content = format!("Validity of word '{}': {}", word, is_valid);
    if is_valid {
        HttpResponse::Accepted().body(content)
    } else {
        HttpResponse::NotAcceptable().body(content)
    }
}

fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(web::resource("/validate_bip39_word/{word}").to(validate_bip39_word))
    })
    .bind("127.0.0.1:8080")?
    .run()
}
