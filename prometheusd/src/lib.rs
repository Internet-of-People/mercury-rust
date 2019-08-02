pub mod daemon;
pub mod data;
pub mod http;
pub mod imp;
mod options;

use std::sync::Mutex;
use std::time::Duration;

use actix_cors::Cors;
use actix_server::Server;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use failure::{err_msg, Fallible};
use log::*;
//use structopt::StructOpt;

pub use crate::daemon::Daemon;
use crate::http::init_url_mapping;
pub use crate::options::Options;
use claims::api::*;
use did::repo::*;
use did::vault::*;
use osg_rpc_storage::RpcProfileRepository;

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