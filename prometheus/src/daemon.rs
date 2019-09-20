use tokio::runtime::current_thread;

use crate::*;

pub struct Daemon {
    rt: current_thread::Runtime,
    server: actix_server::Server,
    join_handle: std::thread::JoinHandle<Fallible<()>>,
}

impl Daemon {
    fn run(options: Options, tx: futures::sync::oneshot::Sender<Server>) -> Fallible<()> {
        let runner = actix_rt::System::builder().name("http-server").build();
        let server = start_daemon(options)?;
        tx.send(server).map_err(|_tx| err_msg("Could not initialize runtime"))?;

        runner.run()?;
        Ok(())
    }

    pub fn start(options: Options) -> Fallible<Self> {
        let (tx, rx) = futures::sync::oneshot::channel();

        let mut rt = current_thread::Runtime::new()?;
        let join_handle =
            std::thread::Builder::new().name("actix-system".to_owned()).spawn(move || {
                let daemon_res = Daemon::run(options, tx);
                match daemon_res {
                    Ok(()) => debug!("Daemon thread exited succesfully"),
                    Err(ref e) => error!("Daemon thread failed: {}", e),
                };
                daemon_res
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
    let mut vault: Option<Box<dyn ProfileVault + Send>> = None;
    if vault_exists {
        info!("Found profile vault, loading {}", vault_path.to_string_lossy());
        vault = Some(Box::new(HdProfileVault::load(&vault_path)?))
    } else {
        info!("No profile vault found in {}, restore it first", vault_path.to_string_lossy());
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
            .wrap(middleware::Logger::default())
            .wrap(Cors::default())
            .data(web::JsonConfig::default().limit(16_777_216))
            .register_data(daemon_state.clone())
            .configure(http::server::mapping::init_url_mapping)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .workers(1) // default is a thread on each CPU core, but we're serving on localhost only
    .system_exit()
    .bind(&options.listen_on)?
    .start();

    Ok(server)
}
