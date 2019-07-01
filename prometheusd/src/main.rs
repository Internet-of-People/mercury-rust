mod options;

use std::sync::Mutex;
use std::time::Duration;

use actix_cors::Cors;
use actix_web::{http::header, middleware, web, App, HttpResponse, HttpServer, Responder};
use failure::{err_msg, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};
use structopt::StructOpt;

use crate::options::Options;
use claims::api::*;
use did::repo::*;
use did::vault::*;
use keyvault::Seed;
use osg_rpc_storage::RpcProfileRepository;

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

fn run_daemon(options: Options) -> Fallible<()> {
    let vault_path = did::paths::vault_path(options.config_dir.clone())?;
    let repo_path = did::paths::profile_repo_path(options.config_dir.clone())?;
    let base_path = did::paths::base_repo_path(options.config_dir.clone())?;

    let vault_exists = vault_path.exists();
    let mut vault: Option<Box<ProfileVault + Send>> = None;
    if vault_exists {
        info!("Found profile vault, loading {}", vault_path.to_string_lossy());
        vault = Some(Box::new(HdProfileVault::load(&vault_path)?))
    } else {
        info!("No profile vault found, it'll need initialization");
    }

    let local_repo = FileProfileRepository::new(&repo_path)?;
    let base_repo = FileProfileRepository::new(&base_path)?;
    let timeout = Duration::from_secs(options.network_timeout_secs);
    let rpc_repo = RpcProfileRepository::new(&options.remote_repo_address, timeout)?;

    let ctx = Context::new(
        vault_path.clone(),
        vault,
        local_repo,
        Box::new(base_repo),
        Box::new(rpc_repo.clone()),
        Box::new(rpc_repo),
    );

    let daemon_state = web::Data::new(Mutex::new(ctx));

    // TODO The current implementation is not known to ever panic. However,
    //      if it was then the Arbiter thread would stop but not the whole server.
    //      The server should not be in an inconsistent half-stopped state after any panic.
    HttpServer::new(move || {
        App::new()
            .data(web::JsonConfig::default().limit(65536))
            .wrap(middleware::Logger::default())
            .wrap(
                Cors::new()
                    .allowed_origin("*")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                    .allowed_header(header::CONTENT_TYPE)
                    .max_age(3600),
            )
            .register_data(daemon_state.clone())
            .service(
                web::scope("/bip39")
                    .service(web::resource("").route(web::post().to(generate_bip39_phrase)))
                    .service(
                        web::resource("/validate_phrase")
                            .route(web::post().to(validate_bip39_phrase)),
                    )
                    .service(
                        web::resource("/validate_word").route(web::post().to(validate_bip39_word)),
                    ),
            )
            .service(
                web::scope("/vault")
                    .service(web::resource("").route(web::post().to(init_vault)))
                    .service(
                        web::resource("/dids")
                            .route(web::get().to(list_did))
                            .route(web::post().to(create_did)),
                    )
                    .service(web::resource("/dids/{did}/alias").route(web::post().to(rename_did))),
            )
            .default_service(web::to(|| HttpResponse::NotFound()))
    })
    .workers(1) // default is a thread on each CPU core, but we're serving on localhost only
    .bind(&options.listen_on)?
    .run()?;

    Ok(())
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
    let phrase_str = Seed::generate_bip39();
    let words = phrase_str.split_whitespace().collect::<Vec<_>>();
    HttpResponse::Ok().json(words)
}

fn validate_bip39_phrase(words: web::Json<Vec<String>>) -> impl Responder {
    let phrase = words.join(" ");
    let is_valid = Seed::from_bip39(&phrase).is_ok();
    HttpResponse::Ok().json(is_valid)
}

fn validate_bip39_word(word: web::Json<String>) -> impl Responder {
    let is_valid = Seed::check_word(&word);
    HttpResponse::Ok().json(is_valid)
}

// TODO this Fallible -> Responder mapping + logging should be somehow less manual
fn init_vault(state: web::Data<Mutex<Context>>, words: web::Json<Vec<String>>) -> impl Responder {
    match init_vault_impl(state, words) {
        Ok(()) => {
            debug!("Initialized vault");
            HttpResponse::Ok().body("")
        }
        Err(e) => {
            error!("Failed to initialize vault: {}", e);
            HttpResponse::BadRequest().body(e.to_string())
        }
    }
}

fn init_vault_impl(
    state: web::Data<Mutex<Context>>,
    words: web::Json<Vec<String>>,
) -> Fallible<()> {
    // TODO state locking also should not be manual in each function
    let mut state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    let phrase = words.join(" ");
    state.restore_vault(phrase)?;
    state.save_vault()
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
struct ProfileEntry {
    id: String,
    alias: String,
    avatar: Vec<u8>,
    state: String,
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

fn list_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<ProfileEntry>> {
    let state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    state.list_profiles().map(|ids| {
        ids.iter()
            .map(|(alias, profile_id)| ProfileEntry {
                id: profile_id.to_string(),
                alias: alias.to_owned(),
                avatar: vec![],
                state: "TODO".to_owned(),
            })
            .collect::<Vec<_>>()
    })
}

fn create_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    match create_dids_impl(state) {
        Ok(entry) => {
            debug!("Created profile {} with alias {}", entry.id, entry.alias);
            HttpResponse::Ok().json(entry)
        }
        Err(e) => {
            error!("Failed to create profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

fn create_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<ProfileEntry> {
    let mut state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    //let alias = state.list_profiles()?.len().to_string();
    // TODO this might provide worse performance than keeping a generator instance in the state,
    //      but that is probably not significant in practice
    let alias = names::Generator::default().next().unwrap_or("FAILING FAILURE".to_owned());
    let did = state.create_profile(alias.clone())?;
    state.save_vault()?;
    Ok(ProfileEntry { id: did.to_string(), alias, avatar: vec![], state: "TODO".to_owned() })
}

fn rename_did(
    state: web::Data<Mutex<Context>>,
    did: web::Path<String>,
    //did: web::Path<ProfileId>,
    name: web::Json<ProfileAlias>,
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

fn rename_did_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
    //did: ProfileId,
    name: ProfileAlias,
) -> Fallible<()> {
    let did = did_str.parse()?;
    let mut state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    state.rename_profile(Some(did), name)?;
    state.save_vault()?;
    Ok(())
}
