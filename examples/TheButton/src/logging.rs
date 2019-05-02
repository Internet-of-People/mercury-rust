use clap::ArgMatches;
use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Logger, Root};
use log4rs::encode::pattern::PatternEncoder;

pub fn start_logging(matches: &ArgMatches) {
    let level = match matches.occurrences_of(crate::cli::CLI_VERBOSE) {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{l} {t}:{L} - {m}{n}")))
        .build();

    let logfile = FileAppender::builder().build("log/button.log").unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("tokio_core", LevelFilter::Warn))
        .logger(Logger::builder().build("tokio_reactor", LevelFilter::Warn))
        .logger(Logger::builder().build("tokio_threadpool", LevelFilter::Warn))
        .logger(Logger::builder().build("mio", LevelFilter::Warn))
        .build(Root::builder().appender("stdout").appender("logfile").build(level))
        .unwrap();

    log4rs::init_config(config).unwrap();
}
