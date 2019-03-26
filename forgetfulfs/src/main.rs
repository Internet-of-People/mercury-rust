mod fs;

use std::ffi::OsStr;

use failure::{err_msg, Fallible};
use log::*;

use fs::ForgetfulFS;

fn init_log(level_filter: LevelFilter) -> Fallible<()> {
    use log4rs::append::console::ConsoleAppender;
    use log4rs::config::{Appender, Config, Root};
    use log4rs::encode::pattern::PatternEncoder;

    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{h({m}{n})}")))
        .build();
    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .build(Root::builder().appender("stdout").build(level_filter))?;

    log4rs::init_config(config)?;
    Ok(())
}

fn main() -> Fallible<()> {
    init_log(LevelFilter::Trace)?;

    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        return Err(err_msg("Not enough parameters"));
    }

    let mount = &args[1];
    info!("forgetfulfs {}", mount);
    let fs = ForgetfulFS::new(users::get_current_uid(), users::get_current_gid());
    let options = [
        OsStr::new("-o"),
        OsStr::new("rootmode=700,auto_unmount,default_permissions,noatime"),
    ];
    let fs_mt = fuse_mt::FuseMT::new(fs, 2);
    fuse_mt::mount(fs_mt, mount, &options[..]).map_err(|e| e.into())
}
