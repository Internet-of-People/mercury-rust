use failure::Fallible;
use log::*;
use prometheusd::{init_logger, run_daemon, Options, StructOpt};

fn main() -> Fallible<()> {
    let options = Options::from_args();
    init_logger(&options)?;

    // NOTE HTTP server already handles signals internally unless the no_signals option is set.
    match std::thread::spawn(move || run_daemon(options)).join() {
        Err(e) => info!("Daemon thread failed with error: {:?}", e),
        Ok(Err(e)) => info!("Web server failed with error: {:?}", e),
        Ok(Ok(())) => info!("Graceful shut down"),
    };

    Ok(())
}
