use std::cell::RefCell;
use std::collections::HashMap;
//use std::fmt::Display;
use std::rc::Rc;

//use failure::Fail; // Backtrace, Context
use futures::future;
use futures::prelude::*;
use log::*;
use tokio_core::reactor;

use super::*;
use did::vault::ProfileVault;
use mercury_home_protocol::net::HomeConnector;
use profile::{MyProfile, MyProfileImpl};
use sdk::DAppSessionImpl;

pub trait AdminSession {
    fn profiles(&self) -> AsyncResult<Vec<Rc<dyn MyProfile>>, Error>;
    fn profile(&self, id: ProfileId) -> AsyncResult<Rc<dyn MyProfile>, Error>;
    fn create_profile(&self) -> AsyncResult<Rc<dyn MyProfile>, Error>;
    fn remove_profile(&self, profile: &ProfileId) -> AsyncResult<(), Error>;
    //    fn claim_profile(&self, home: ProfileId, profile: ProfileId)
    //        -> AsyncResult<Rc<MyProfile>, Error>;
}

pub struct SignerFactory {
    // TODO this should also support HW wallets
    signers: HashMap<ProfileId, Rc<dyn Signer>>,
}

impl SignerFactory {
    pub fn new(signers: HashMap<ProfileId, Rc<dyn Signer>>) -> Self {
        Self { signers }
    }

    pub fn signer(&self, profile_id: &ProfileId) -> Option<Rc<dyn Signer>> {
        self.signers.get(profile_id).map(|s| s.clone())
    }
}

pub struct MyProfileFactory {
    signer_factory: Rc<SignerFactory>,
    profile_repo: Rc<RefCell<dyn DistributedPublicProfileRepository>>,
    home_connector: Rc<dyn HomeConnector>,
    handle: reactor::Handle,
    cache: Rc<RefCell<HashMap<ProfileId, Rc<dyn MyProfile>>>>,
}

// TODO maybe this should be merged into AdminSessionImpl, the only thing it does is caching
impl MyProfileFactory {
    //pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<ProfileRepo>, home_connector: Rc<HomeConnector>)
    pub fn new(
        signer_factory: Rc<SignerFactory>,
        profile_repo: Rc<RefCell<dyn DistributedPublicProfileRepository>>,
        home_connector: Rc<dyn HomeConnector>,
        handle: reactor::Handle,
    ) -> Self {
        Self { signer_factory, profile_repo, home_connector, handle, cache: Default::default() }
    }

    pub fn create(&self, own_profile: OwnProfile) -> Result<Rc<dyn MyProfile>, Error> {
        let profile_id = own_profile.id();
        if let Some(ref my_profile_rc) = self.cache.borrow().get(&profile_id) {
            return Ok(Rc::clone(my_profile_rc));
        }

        debug!("Creating new profile wrapper for profile {}", profile_id);
        self.signer_factory
            .signer(&profile_id)
            .map(|signer| {
                let result = MyProfileImpl::new(
                    own_profile,
                    signer,
                    self.profile_repo.clone(),
                    self.home_connector.clone(),
                    self.handle.clone(),
                );
                let result_rc = Rc::new(result) as Rc<dyn MyProfile>;
                // TODO this allows initiating several fill attempts in parallel
                //      until first one succeeds, last one wins by overwriting.
                //      Is this acceptable?
                self.cache.borrow_mut().insert(profile_id, result_rc.clone());
                result_rc
            })
            .ok_or(ErrorKind::FailedToAuthorize.into())
    }
}

pub struct AdminSessionImpl {
    profile_vault: Rc<dyn ProfileVault>,
    profile_repo: Rc<RefCell<dyn PrivateProfileRepository>>,
    profile_factory: Rc<MyProfileFactory>,
    //    handle:         reactor::Handle,
}

impl AdminSessionImpl {
    #[deprecated]
    pub fn new(
        profile_vault: Rc<dyn ProfileVault>,
        profile_repo: Rc<RefCell<dyn PrivateProfileRepository>>,
        profile_factory: Rc<MyProfileFactory>,
    ) -> Rc<dyn AdminSession> {
        let this = Self { profile_vault, profile_repo, profile_factory }; //, handle };
        Rc::new(this)
    }
}

impl AdminSession for AdminSessionImpl {
    fn profiles(&self) -> AsyncResult<Vec<Rc<dyn MyProfile>>, Error> {
        // TODO consider delegating implementation to profile(id)
        let store = self.profile_repo.clone();
        let prof_factory = self.profile_factory.clone();
        let profiles = match self.profile_vault.profiles() {
            Ok(profiles) => profiles,
            Err(_e) => return Box::new(future::err(ErrorKind::FailedToLoadProfile.into())),
        };

        let profile_futs = profiles
            .iter()
            .map(|profile| {
                let prof_factory = prof_factory.clone();
                store
                    .borrow()
                    .get(&profile.id())
                    .map_err(|e| e.context(ErrorKind::FailedToLoadProfile).into())
                    .and_then(move |own_profile| prof_factory.create(own_profile))
            })
            .collect::<Vec<_>>();
        let profiles_fut = future::join_all(profile_futs);
        Box::new(profiles_fut)
    }

    fn profile(&self, id: ProfileId) -> AsyncResult<Rc<dyn MyProfile>, Error> {
        let profile_factory = self.profile_factory.clone();
        let fut = self
            .profile_repo
            .borrow()
            .get(&id)
            .map_err(|e| e.context(ErrorKind::FailedToLoadProfile).into())
            .and_then(move |own_profile| profile_factory.create(own_profile));
        Box::new(fut)
    }

    fn create_profile(&self) -> AsyncResult<Rc<dyn MyProfile>, Error> {
        unimplemented!()
    }

    //    fn claim_profile(&self, home_id: ProfileId, profile: ProfileId) ->
    //        AsyncResult<Rc<MyProfile>, Error>
    //    {
    //        let claim_fut = self.connect_home(&home_id)
    //            .map_err(|err| err.context(ErrorKind::ConnectionToHomeFailed).into())
    //            .and_then( move |home| {
    //                home.claim(profile)
    //                    .map_err(|err| err.context(ErrorKind::FailedToClaimProfile).into())
    //            });
    //        Box::new(claim_fut)
    //    }

    fn remove_profile(&self, _profile: &ProfileId) -> AsyncResult<(), Error> {
        unimplemented!()
    }
}

pub struct ConnectService {
    profile_vault: Rc<dyn ProfileVault>,
    profile_repo: Rc<RefCell<dyn PrivateProfileRepository>>,
    profile_factory: Rc<MyProfileFactory>,
    //    handle:         reactor::Handle,
}

impl ConnectService {
    pub fn new(
        profile_vault: Rc<dyn ProfileVault>,
        profile_repo: Rc<RefCell<dyn PrivateProfileRepository>>,
        profile_factory: Rc<MyProfileFactory>,
    ) -> Self {
        Self { profile_vault, profile_repo, profile_factory }
    } //, handle: handle.clone() } }

    pub fn admin_session(
        &self,
        _authorization: Option<DAppPermission>,
    ) -> AsyncResult<Rc<dyn AdminSession>, Error> {
        let adm = AdminSessionImpl::new(
            self.profile_vault.clone(),
            self.profile_repo.clone(),
            self.profile_factory.clone(),
        ); //, self.handle.clone() );

        Box::new(Ok(adm).into_future())
    }
}

impl DAppEndpoint for ConnectService {
    fn dapp_session(
        &self,
        app: &ApplicationId,
        _authorization: Option<DAppPermission>,
    ) -> AsyncResult<Rc<dyn DAppSession>, Error> {
        let app = app.to_owned();
        let profile_store = self.profile_repo.clone();
        let profile_factory = self.profile_factory.clone();
        // TODO user should be able to manually pair a profile with the app
        let profile_id_res = self
            .profile_vault
            .get_active()
            .map_err(|_e| ErrorKind::FailedToGetSession.into())
            .and_then(|profile_id_opt| profile_id_opt.ok_or(ErrorKind::FailedToGetSession.into()))
            .into_future();
        let fut = profile_id_res
            .and_then(move |profile_id| {
                let store = profile_store.borrow();
                store
                    .get(&profile_id)
                    .map_err(|err| err.context(ErrorKind::FailedToLoadProfile).into())
            })
            .and_then(move |own_profile| profile_factory.create(own_profile))
            .map(move |my_profile| DAppSessionImpl::new(my_profile, app))
            .map_err(|err| {
                debug!("Failed to initialize dapp session: {:?}", err);
                err
            });
        Box::new(fut)
    }
}
