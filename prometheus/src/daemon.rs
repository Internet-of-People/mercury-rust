use failure::format_err;
use futures01::prelude::*;
use tokio01::runtime::current_thread;
use tokio01::sync::oneshot;

use crate::dapp::dapp_session::DAppSessionServiceImpl;
use crate::home::discovery::HomeNodeCrawler;
use crate::home::net::{HomeConnector, TcpHomeConnector};
use crate::test::FakeUserInteractor;
use crate::vault::api_impl::VaultState;
use crate::*;

pub struct Daemon {
    handle: current_thread::Handle,
    http_server: Server,
    join_handle: std::thread::JoinHandle<Fallible<()>>,
}

impl Daemon {
    fn run(
        options: Options,
        tx_initialized: oneshot::Sender<(current_thread::Handle, Server)>,
    ) -> Fallible<()> {
        let mut reactor = current_thread::Runtime::new()?;
        let handle = reactor.handle();
        let executor = tokio_current_thread::CurrentThread::new().handle();
        let actix_runner = actix_rt::System::run_in_executor("http-server", executor);
        let server = start_daemon(options)?;
        tx_initialized
            .send((handle, server))
            .map_err(|_tx| err_msg("Could not initialize runtime"))?;
        reactor.block_on(actix_runner)?;
        Ok(())
    }

    pub fn start(options: Options) -> Fallible<Self> {
        let (tx_initialized, rx_initialized) = oneshot::channel();
        let mut reactor = current_thread::Runtime::new()?;

        let join_handle =
            std::thread::Builder::new().name("actix-system".to_owned()).spawn(move || {
                let daemon_res = Daemon::run(options, tx_initialized);
                match daemon_res {
                    Ok(()) => debug!("Daemon thread exited successfully"),
                    Err(ref e) => error!("Daemon thread failed: {}", e),
                };
                daemon_res
            })?;
        let (handle, server) = reactor.block_on(rx_initialized)?;

        Ok(Self { handle, http_server: server, join_handle })
    }

    pub fn stop(&mut self) -> Fallible<()> {
        trace!("before stop");
        let stop_fut01 =
            self.http_server.stop(true).map_err(|()| error!("Could not stop server gracefully"));
        self.handle.spawn(stop_fut01)?;
        trace!("after stop");
        Ok(())
    }

    pub fn join(self) -> Fallible<()> {
        trace!("before join");
        self.join_handle.join().map_err(|_e| err_msg("Thread panicked"))??;
        trace!("after join");
        Ok(())
    }
}

pub struct NetworkState {
    pub home_connector: Arc<RwLock<dyn HomeConnector + Send + Sync>>,
    //pub explorer: Box<dyn ProfileExplorer + Send>,
    pub home_node_crawler: Arc<RwLock<HomeNodeCrawler>>,
}

impl NetworkState {
    pub fn new(
        home_connector: Arc<RwLock<dyn HomeConnector + Send + Sync>>,
        //explorer: Box<dyn ProfileExplorer + Send>,
        home_node_crawler: Arc<RwLock<HomeNodeCrawler>>,
    ) -> Self {
        Self { home_connector, home_node_crawler }
    }

    pub fn homes(&self) -> Fallible<Vec<HomeNode>> {
        let crawler = self
            .home_node_crawler
            .try_read()
            .map_err(|e| format_err!("Failed to lock crawler: {}", e))?;
        let homes = crawler.iter().map(|known_home_node| known_home_node.into()).collect();
        Ok(homes)
    }
}

pub struct DaemonState {
    pub vault: VaultState,
    pub dapp: DAppSessionServiceImpl,
    pub network: NetworkState,
}

impl DaemonState {
    fn new(vault: VaultState, dapp: DAppSessionServiceImpl, network: NetworkState) -> Self {
        Self { vault, dapp, network }
    }
}

fn start_daemon(options: Options) -> Fallible<Server> {
    let vault_path = did::paths::vault_path(options.config_dir.clone())?;
    let repo_path = did::paths::profile_repo_path(options.config_dir.clone())?;
    let base_path = did::paths::base_repo_path(options.config_dir.clone())?;
    let schema_path = did::paths::schemas_path(options.schemas_dir.clone())?;

    let mut interactor = FakeUserInteractor::new();

    let vault_exists = vault_path.exists();
    let mut vault: Option<Arc<dyn ProfileVault + Send + Sync>> = None;
    if vault_exists {
        info!("Found profile vault, loading {}", vault_path.to_string_lossy());
        let hd_vault = HdProfileVault::load(&vault_path)?;
        interactor.set_active_profile(hd_vault.get_active()?);
        vault = Some(Arc::new(hd_vault));
    } else {
        info!("No profile vault found in {}, restore it first", vault_path.to_string_lossy());
    }

    let local_repo = FileProfileRepository::new(&repo_path)?;
    let base_repo = FileProfileRepository::new(&base_path)?;
    let timeout = Duration::from_secs(options.network_timeout_secs);
    // TODO use some kind of real storage here on the long run
    let remote_repo =
        FileProfileRepository::new(&std::path::PathBuf::from("/tmp/mercury/home/profile-backups"))?;
    let home_node_crawler = Default::default();

    let vault_state = VaultState::new(
        vault_path.clone(),
        schema_path.clone(),
        vault,
        Arc::new(RwLock::new(local_repo)),
        Box::new(base_repo),
        Box::new(remote_repo),
    );

    // TODO make file path configurable, check config parameters for potential outdated repo path
    // TODO use crawler and connected home nodes for distributed storage on the long run
    let profile_repo = Arc::new(RwLock::new(FileProfileRepository::new(
        &std::path::PathBuf::from("/tmp/cuccos"),
    )?));
    let connector = Arc::new(RwLock::new(TcpHomeConnector::new(profile_repo.clone())));
    let dapp_state = DAppSessionServiceImpl::new(Arc::new(RwLock::new(interactor)));

    let network_state = NetworkState::new(connector, home_node_crawler);

    let daemon_state =
        web::Data::new(Mutex::new(DaemonState::new(vault_state, dapp_state, network_state)));

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
