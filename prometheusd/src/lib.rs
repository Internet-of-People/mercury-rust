mod options;

use std::sync::Mutex;
use std::time::Duration;

use actix_cors::Cors;
use actix_server::Server;
use actix_web::{middleware, web, App, HttpResponse, HttpServer, Responder};
use failure::{err_msg, Fallible};
use log::*;
use serde::Serializer;
use serde_derive::{Deserialize, Serialize};
pub use structopt::StructOpt;
use tokio::runtime::current_thread;

pub use crate::options::Options;
use claims::api::*;
use did::repo::*;
use did::vault::*;
use keyvault::Seed;
use osg_rpc_storage::RpcProfileRepository;

pub struct Daemon {
    rt: current_thread::Runtime,
    server: actix_server::Server,
    join_handle: std::thread::JoinHandle<Fallible<()>>,
}

impl Daemon {
    pub fn start(options: Options) -> Fallible<Self> {
        let (tx, rx) = futures::sync::oneshot::channel();

        let mut rt = current_thread::Runtime::new()?;
        let join_handle =
            std::thread::Builder::new().name("actix-system".to_owned()).spawn(move || {
                let runner = actix_rt::System::builder().name("http-server").build();
                let server = start_daemon(options)?;
                tx.send(server).map_err(|_tx| err_msg("Could not initialize runtime"))?;

                runner.run().map_err(|e| e.into())
            })?;
        let server = rt.block_on(rx)?;

        Ok(Self { rt, server, join_handle })
    }

    pub fn stop(&mut self) -> Fallible<()> {
        trace!("before stop");
        self.rt
            .block_on(self.server.stop(true))
            .map_err(|()| err_msg("Could not stop server gracefully"))?;
        trace!("after stop");
        Ok(())
    }

    pub fn join(self) -> Fallible<()> {
        trace!("before join");
        self.join_handle.join().map_err(|_e| err_msg("Thread panicked")).and_then(|r| r)?;
        trace!("after join");
        Ok(())
    }
}

fn start_daemon(options: Options) -> Fallible<Server> {
    let vault_path = did::paths::vault_path(options.config_dir.clone())?;
    let repo_path = did::paths::profile_repo_path(options.config_dir.clone())?;
    let base_path = did::paths::base_repo_path(options.config_dir.clone())?;
    let schema_path = did::paths::schemas_path(options.schemas_dir.clone())?;

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
        schema_path.clone(),
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
    let server = HttpServer::new(move || {
        App::new()
            .data(web::JsonConfig::default().limit(65536))
            .wrap(middleware::Logger::default())
            .wrap(Cors::default())
            .register_data(daemon_state.clone())
            .service(
                web::scope("/bip39")
                    .service(web::resource("").route(web::post().to(generate_bip39_phrase)))
                    .service(
                        web::resource("/validate-phrase")
                            .route(web::post().to(validate_bip39_phrase)),
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
                        .service(web::resource("")
                            .route(web::get().to(list_did))
                            .route(web::post().to(create_did))
                        )
                        .service(web::resource("/{did}").route(web::get().to(get_did)))
                        .service(web::resource("/{did}/alias").route(web::put().to(rename_did)))
                        .service(web::resource("/{did}/avatar").route(web::put().to(set_avatar))),
                    )
                    .service(
                        web::scope("/claims")
                        .service(web::resource("").route(web::to(HttpResponse::NotFound)))
                    )
            )
            .service(web::resource("/claim-schemas").route(web::get().to(list_schemas)))
            .default_service(web::to(HttpResponse::NotFound))
    })
    .workers(1) // default is a thread on each CPU core, but we're serving on localhost only
    .system_exit()
    .bind(&options.listen_on)?
    .start();

    Ok(server)
}

pub fn init_logger(options: &Options) -> Fallible<()> {
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
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))?;

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
            HttpResponse::Created().body("")
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
struct VaultEntry {
    id: String,
    alias: String,
    #[serde(serialize_with = "serialize_avatar")] //, deserialize_with = "deserialize_avatar")]
    avatar: Vec<u8>,
    state: String,
}

impl From<&ProfileVaultRecord> for VaultEntry {
    fn from(src: &ProfileVaultRecord) -> Self {
        VaultEntry {
            id: src.id().to_string(),
            alias: src.alias(),
            avatar: src.metadata(),
            state: "TODO".to_owned(), // TODO this will probably need another query to context
        }
    }
}

pub fn serialize_avatar<S: Serializer>(avatar: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    // About the format used here, see https://en.wikipedia.org/wiki/Data_URI_scheme
    let data_uri = format!("data:image/png;base64,{}", base64::encode(avatar));
    serializer.serialize_str(&data_uri)
}

//pub fn deserialize_avatar<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
//    use serde::{de, Deserialize};
//    let data_uri = String::deserialize(deserializer)?;
//
//    // TODO do we need support for more encodings than just base64 here?
//    let re = regex::Regex::new(r"(?x)data:image/(?P<format>\w+);base64,(?P<data>.*)")
//        .map_err(de::Error::custom)?;
//    let captures = re
//        .captures(&data_uri)
//        .ok_or_else(|| de::Error::custom("Provided image is not in DataURI format"))?;
//    let (avatar_format, encoded_avatar) = (&captures["format"], &captures["data"]);
//    let avatar = base64::decode(encoded_avatar).map_err(de::Error::custom)?;
//    Ok(avatar)
//}

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

fn list_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<VaultEntry>> {
    let state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    state.list_vault_records().map(|recs| recs.iter().map(VaultEntry::from).collect::<Vec<_>>())
}

fn create_did(state: web::Data<Mutex<Context>>) -> impl Responder {
    match create_dids_impl(state) {
        Ok(entry) => {
            debug!("Created profile {} with alias {}", entry.id, entry.alias);
            HttpResponse::Created().json(entry)
        }
        Err(e) => {
            error!("Failed to create profile: {}", e);
            HttpResponse::Conflict().body(e.to_string())
        }
    }
}

fn create_dids_impl(state: web::Data<Mutex<Context>>) -> Fallible<VaultEntry> {
    let mut state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;

    //let alias = state.list_profiles()?.len().to_string();
    // TODO this name generation is not deterministic, but should be (found no proper lib)
    // TODO instantiating generators here might provide worse performance than keeping
    //      a generator instance in the state, but that is probably not significant in practice
    let alias = names::Generator::default().next().unwrap_or_else(|| "FAILING FAILURE".to_owned());
    let did = state.create_profile(alias.clone())?;

    let mut avatar_png = Vec::new();
    blockies::Ethereum::default()
        .create_icon(&mut avatar_png, &did.to_bytes())
        .map_err(|e| err_msg(format!("Failed to generate default profile icon: {:?}", e)))?;
    //std::fs::write(format!("/tmp/{}.png", alias), &avatar_png)?;
    state.set_profile_metadata(Some(did.clone()), avatar_png.clone())?;

    state.save_vault()?;
    Ok(VaultEntry { id: did.to_string(), alias, avatar: avatar_png, state: "TODO".to_owned() })
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

fn get_did_impl(state: web::Data<Mutex<Context>>, did_str: String) -> Fallible<VaultEntry> {
    let did = did_str.parse()?;
    let state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    let rec = state.get_vault_record(Some(did))?;
    Ok(VaultEntry::from(&rec))
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

type DataUri = String;
type ImageFormat = String;
type ImageBlob = Vec<u8>;

// TODO do we need support for more encodings than just base64 here?
pub fn parse_avatar(data_uri: &str) -> Fallible<(ImageFormat, ImageBlob)> {
    let re = regex::Regex::new(r"(?x)data:image/(?P<format>\w+);base64,(?P<data>.*)")?;
    let captures = re
        .captures(&data_uri)
        .ok_or_else(|| err_msg("Provided image is not in DataURI format, see https://en.wikipedia.org/wiki/Data_URI_scheme"))?;
    let (format, encoded_avatar) = (&captures["format"], &captures["data"]);
    let avatar = base64::decode(encoded_avatar)?;
    Ok((format.to_owned(), avatar))
}

fn set_avatar_impl(
    state: web::Data<Mutex<Context>>,
    did_str: String,
    avatar_datauri: DataUri,
) -> Fallible<()> {
    let did = did_str.parse()?;
    // TODO we should also save format with the binary image
    let (format, avatar_binary) = parse_avatar(&avatar_datauri)?;
    let mut state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    state.set_profile_metadata(Some(did), avatar_binary)?;
    state.save_vault()?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct ClaimSchema {
    id: String,
    alias: String,
    content: serde_json::Value,
    ordering: Vec<String>,
}

impl ClaimSchema {
    fn new(
        id: impl ToString,
        alias: impl ToString,
        content: serde_json::Value,
        ordering: Vec<String>,
    ) -> Self {
        Self { id: id.to_string(), alias: alias.to_string(), content, ordering }
    }
}

impl From<&claims::claim_schema::SchemaVersion> for ClaimSchema {
    fn from(model: &claims::claim_schema::SchemaVersion) -> Self {
        Self::new(model.id(), model.name(), model.content().clone(), model.ordering().to_vec())
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

fn list_schemas_impl(state: web::Data<Mutex<Context>>) -> Fallible<Vec<ClaimSchema>> {
    let state = state.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))?;
    let repo = state.claim_schemas()?;
    Ok(repo.iter().map(|v| v.into()).collect::<Vec<_>>())
}
