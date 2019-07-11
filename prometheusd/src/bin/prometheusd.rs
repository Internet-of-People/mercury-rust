use failure::Fallible;
use log::*;
use prometheusd::{init_logger, Daemon, Options, StructOpt};

fn main() -> Fallible<()> {
    let options = Options::from_args();

    init_logger(&options)?;

    let daemon = Daemon::start(options)?;

    // let registry = prometheusd::ClaimSchemaRegistry::import_folder(&std::path::PathBuf::from("./schemas"))?;
    // for (_k, v) in registry.schemas {
    //     info!("***\n{:#?}\n***", v);
    // }

    // NOTE HTTP server already handles signals internally unless the no_signals option is set.
    match daemon.join() {
        Err(e) => info!("Daemon thread failed with error: {:?}", e),
        Ok(_) => info!("Graceful shut down"),
    };

    Ok(())
}
