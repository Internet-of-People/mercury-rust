use futures::Future;
use tokio_current_thread as reactor;

use crate::dapp::dapp_session::DAppSessionServiceImpl;
use crate::dapp::user_interactor::UserInteractor;
use crate::home::net::TcpHomeConnector;
use crate::test::FakeUserInteractor;
use crate::vault::api_impl::VaultState;
use crate::*;
use claims::repo::DistributedPublicProfileRepository;

pub struct Daemon {
    handle: reactor::Handle,
    http_server: Server,
    join_handle: std::thread::JoinHandle<Fallible<()>>,
}

impl Daemon {
    fn run(
        options: Options,
        tx: futures::sync::oneshot::Sender<(reactor::Handle, Server)>,
    ) -> Fallible<()> {
        let mut reactor = reactor::CurrentThread::new();
        let handle = reactor.handle();
        let actix_runner = actix_rt::System::run_in_executor("http-server", reactor.handle());
        let server = start_daemon(options)?;
        tx.send((handle, server)).map_err(|_tx| err_msg("Could not initialize runtime"))?;
        reactor.block_on(actix_runner)?;
        Ok(())
    }

    pub fn start(options: Options) -> Fallible<Self> {
        let (tx, rx) = futures::sync::oneshot::channel();

        let join_handle =
            std::thread::Builder::new().name("actix-system".to_owned()).spawn(move || {
                let daemon_res = Daemon::run(options, tx);
                match daemon_res {
                    Ok(()) => debug!("Daemon thread exited succesfully"),
                    Err(ref e) => error!("Daemon thread failed: {}", e),
                };
                daemon_res
            })?;
        let (handle, server) = rx.wait()?;

        Ok(Self { handle, http_server: server, join_handle })
    }

    pub fn stop(&mut self) -> Fallible<()> {
        trace!("before stop");
        let stop_fut =
            self.http_server.stop(true).map_err(|()| error!("Could not stop server gracefully"));
        self.handle.spawn(stop_fut)?;
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

pub struct DaemonState {
    vault: Mutex<VaultState>,
    dapp: Mutex<DAppSessionServiceImpl>,
}

impl DaemonState {
    fn new(vault: Mutex<VaultState>, dapp: Mutex<DAppSessionServiceImpl>) -> Self {
        Self { vault, dapp }
    }

    pub fn lock_vault(&self) -> Fallible<MutexGuard<VaultState>> {
        self.vault.lock().map_err(|e| err_msg(format!("Failed to lock state: {}", e)))
    }
}

fn start_daemon(options: Options) -> Fallible<Server> {
    let vault_path = did::paths::vault_path(options.config_dir.clone())?;
    let repo_path = did::paths::profile_repo_path(options.config_dir.clone())?;
    let base_path = did::paths::base_repo_path(options.config_dir.clone())?;
    let schema_path = did::paths::schemas_path(options.schemas_dir.clone())?;

    let mut interactor = FakeUserInteractor::new();

    let vault_exists = vault_path.exists();
    let mut vault: Option<Box<dyn ProfileVault + Send>> = None;
    if vault_exists {
        info!("Found profile vault, loading {}", vault_path.to_string_lossy());
        let hd_vault = HdProfileVault::load(&vault_path)?;
        interactor.set_active_profile(hd_vault.get_active()?);
        vault = Some(Box::new(hd_vault));
    } else {
        info!("No profile vault found in {}, restore it first", vault_path.to_string_lossy());
    }

    let local_repo = FileProfileRepository::new(&repo_path)?;
    let base_repo = FileProfileRepository::new(&base_path)?;
    let timeout = Duration::from_secs(options.network_timeout_secs);
    let rpc_repo = RpcProfileRepository::new(&options.remote_repo_address, timeout)?;

    let vault_state = VaultState::new(
        vault_path.clone(),
        schema_path.clone(),
        vault,
        local_repo,
        Box::new(base_repo),
        Box::new(rpc_repo.clone()),
        Box::new(rpc_repo),
    );

    let profile_repo = Arc::new(RwLock::new(FileProfileRepository::new(&repo_path)?));
    let connector = Arc::new(RwLock::new(TcpHomeConnector::new(profile_repo.clone())));
    let dapp_state =
        DAppSessionServiceImpl::new(Arc::new(RwLock::new(interactor)), connector, profile_repo);
    let daemon_state =
        web::Data::new(DaemonState::new(Mutex::new(vault_state), Mutex::new(dapp_state)));

    // TODO The current implementation is not known to ever panic. However,
    //      if it was then the Arbiter thread would stop but not the whole server.
    //      The server should not be in an inconsistent half-stopped state after any panic.
    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(Cors::default())
            .data(web::JsonConfig::default().limit(16_777_216))
            .register_data(daemon_state.clone())
            .configure(vault::http::server::routes::init_url_mapping)
            .configure(dapp::websocket::routes::init_url_mapping)
            .default_service(web::to(HttpResponse::NotFound))
    })
    .workers(1) // default is a thread on each CPU core, but we're serving on localhost only
    .system_exit()
    .bind(&options.listen_on)?
    .start();

    Ok(server)
}
