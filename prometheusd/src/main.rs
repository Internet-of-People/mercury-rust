mod options;

use std::sync::Mutex;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use failure::Fallible;
use log::*;
use structopt::StructOpt;

use crate::options::Options;
use keyvault::Seed;

fn main() -> Fallible<()> {
    let options = Options::from_args();
    init_logger(&options)?;

    // NOTE HTTP server already handles signals internally unless the no_signals option is set.
    match std::thread::spawn(move || run_daemon(options)).join() {
        Err(e) => info!("Daemon thread failed with error: {:?}", e),
        Ok(Err(e)) => info!("Web server failed with error: {:?}", e),
        Ok(Ok(())) => info!("Gracefully shut down"),
    };

    Ok(())
}

fn run_daemon(options: Options) -> std::io::Result<()> {
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
    .bind(&options.listen_on)?
    .run()
}

fn init_logger(options: &Options) -> Fallible<()> {
    if log4rs::init_file(&options.logger_config, Default::default()).is_err() {
        println!(
            "Failed to initialize loggers from {:?}, using default config",
            options.logger_config
        );

        use log4rs::append::console::ConsoleAppender;
        use log4rs::config::{Appender, Config, Root};
        use log4rs::encode::pattern::PatternEncoder;

        let stdout =
            ConsoleAppender::builder().encoder(Box::new(PatternEncoder::new("{m}{n}"))).build();
        let config = Config::builder()
            .appender(Appender::builder().build("stdout", Box::new(stdout)))
            .build(Root::builder().appender("stdout").build(log::LevelFilter::Info))?;

        log4rs::init_config(config)?;
    };
    Ok(())
}

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
