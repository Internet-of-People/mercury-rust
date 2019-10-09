mod options;
mod seed;

use failure::Fallible;
use log::*;
use structopt::StructOpt;

use crate::options::{Command, Options};
use prometheus::http::client::VaultClient;

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => eprintln!("Error: {}", e),
    };
}

fn run() -> Fallible<()> {
    let options = Options::from_args();
    init_logger(&options)?;

    let command = options.command;
    debug!("Got command {:?}", command);

    let mut client = VaultClient::new(&format!("http://{}", options.prometheus_address));
    let command = Box::new(command);
    command.execute(&mut client)
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
            .build(Root::builder().appender("stdout").build(LevelFilter::Info))?;

        log4rs::init_config(config)?;
    };
    Ok(())
}
