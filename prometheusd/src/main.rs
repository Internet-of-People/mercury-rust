use std::sync::Mutex;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};

use keyvault::Seed;

fn generate_bip39_phrase() -> impl Responder {
    Seed::generate_bip39()
}

fn validate_bip39_phrase(phrase: web::Path<String>) -> impl Responder {
    match Seed::from_bip39(&phrase as &str) {
        Ok(_seed) => HttpResponse::Accepted().body(""),
        Err(e) => HttpResponse::NotAcceptable().body(format!("{}", e)),
    }
}

fn validate_bip39_word(word: web::Path<String>) -> impl Responder {
    let is_valid = Seed::check_word(&word);
    let result = if is_valid { HttpResponse::Accepted() } else { HttpResponse::NotAcceptable() };
    result
}

// TODO to be replaced with the Prometheus command Context
struct MyState {
    counter: u32,
}

impl MyState {
    fn new() -> Self {
        Self { counter: 0 }
    }
}

fn test_state(state: web::Data<Mutex<MyState>>) -> impl Responder {
    let mut state = state.lock().unwrap();
    state.counter += 1;
    format!("{}", state.counter)
}

fn run_daemon(listen_on: &str) -> std::io::Result<()> {
    let daemon_state = web::Data::new(Mutex::new(MyState::new()));

    HttpServer::new(move || {
        App::new()
            .register_data(daemon_state.clone())
            .service(
                web::scope("/vault")
                    .service(web::resource("/generate_phrase").to(generate_bip39_phrase))
                    .service(web::resource("/validate_phrase/{phrase}").to(validate_bip39_phrase))
                    .service(web::resource("/validate_word/{word}").to(validate_bip39_word)),
            )
            .service(web::resource("/test_state").to(test_state))
            .default_service(web::to(|| HttpResponse::NotFound()))
    })
    .bind(listen_on)?
    .run()
}

fn main() {
    let address = "127.0.0.1:8080";
    println!("Listening on {}", address);

    // NOTE HTTP server already handles signals internally unless the no_signals option is set.
    match std::thread::spawn(move || run_daemon(address)).join() {
        Err(e) => println!("Daemon thread failed with error: {:?}", e),
        Ok(Err(e)) => println!("Web server failed with error: {:?}", e),
        Ok(Ok(())) => println!("Gracefully shut down"),
    };
}
