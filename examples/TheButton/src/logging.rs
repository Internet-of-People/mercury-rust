use log::LevelFilter;
use log4rs;
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::encode::pattern::PatternEncoder;
use log4rs::config::{Appender, Config, Logger, Root};

pub fn start_logging(levelstr : &str) {
    let level = match levelstr{
        "o"=>LevelFilter::Off,
        "e"=>LevelFilter::Error,
        "i"=>LevelFilter::Info,
        "w"=>LevelFilter::Warn,
        "d"=>LevelFilter::Debug,
        "t"|_=>LevelFilter::Trace,
    }; 
    let stdout = ConsoleAppender::builder().build();

    let logfile = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d} - {m}{n}")))
        .build("log/button.log")
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .logger(Logger::builder().build("tokio_core::reactor", LevelFilter::Warn))
        .logger(Logger::builder().build("tokio_reactor", LevelFilter::Warn))
        .build(Root::builder().appender("stdout").appender("logfile").build(level))
        .unwrap();

    log4rs::init_config(config).unwrap();
}