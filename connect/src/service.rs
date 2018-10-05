use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
//use std::fmt::Display;
use std::rc::Rc;

use failure::Fail; // Backtrace, Context
use futures::prelude::*;
use futures::future;
use tokio_core::reactor;

use super::*;
use profile::{HomeConnector, MyProfile, MyProfileImpl};
use sdk::DAppConnect;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DeviceAuthorization(Vec<u8>);

// TODO should be used in parsed version as something like Vec<(bool,uint16)>
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Bip32Path(String);



// Hierarchical deterministic seed for identity handling to generate profiles
pub trait KeyVault
{
    // Get the next hierarchical path to generate a new profile with
    fn next(&self) -> Bip32Path;

    // TODO what do we need here to unlock the private key? Maybe a password?
    // Get or create an empty profile for a path returned by next()
    fn unlock_profile(&self, bip32_path: &Bip32Path) -> Rc<Signer>;
}


// Usage of Bip32 hierarchy, format: path => data stored with that key
pub trait Bip32PathMapper
{
    // master_seed/purpose_mercury => last_profile_number and profile {id: number} map
    fn root_path(&self) -> Bip32Path;

    // m/mercury/profile_number => list of relations, apps, etc
    fn profile_path(&self, profile_id: &ProfileId) -> Bip32Path;

    // m/mercury/profile/app_id => application-specific data
    fn app_path(&self, profile_id: &ProfileId, app_id: &ApplicationId) -> Bip32Path;
}


pub trait AccessManager
{
    fn ask_read_access(&self, resource: &Bip32Path) ->
        Box< Future<Item=PublicKey, Error=Error> >;

    fn ask_write_access(&self, resource: &Bip32Path) ->
        Box< Future<Item=Rc<Signer>, Error=Error> >;
}



// User interface (probably implemented with platform-native GUI) for actions
// that are initiated by the SDK and require some kind of user interaction
pub trait UserInterface
{
    // Initialize system components and configuration where user interaction is needed,
    // e.g. HD wallets need manually saving generated new seed or entering old one
    fn initialize(&self) -> Box< Future<Item=(), Error=Error> >;

    // An action requested by a distributed application needs
    // explicit user confirmation.
    // TODO how to show a human-readable summary of the action (i.e. binary to be signed)
    //      making sure it's not a fake/misinterpreted description?
    fn confirm_dappaction(&self, action: &DAppAction)
        -> Box< Future<Item=(), Error=Error> >;

    fn confirm_pairing(&self, request: &RelationHalfProof)
        -> Box< Future<Item=(), Error=Error>>;

    fn notify_pairing(&self, response: &RelationProof)
        -> Box< Future<Item=(), Error=Error>>;

    // Select a profile to be used by a dApp. It can be either an existing one
    // or the user can create a new one (using a KeyVault) to be selected.
    // TODO this should open something nearly identical to manage_profiles()
    fn select_profile(&self)
        -> Box< Future<Item=ProfileId, Error=Error> >;

    // Open profiles with new, delete and edit (e.g. homes, contacts, apps, etc) options.
    // Specific profiles can also be set online/offline.
    // TODO it could look something like:
    //      Profiles
    //      [x]ON  business (edit) (delete)
    //      [ ]off family   (edit) (delete)
    //      [x]ON  hobby    (edit) (delete)
    //      (new profile)
    fn manage_profiles(&self)
        -> Box< Future<Item=(), Error=Error> >;
}



pub trait AdminSession
{
    fn profiles(&self) -> Box< Future<Item=Vec<Rc<MyProfile>>, Error=Error> >;
    fn profile(&self, id: ProfileId) -> Box< Future<Item=Rc<MyProfile>, Error=Error> >;
    fn create_profile(&self) -> Box< Future<Item=Rc<MyProfile>, Error=Error> >;
    fn remove_profile(&self, profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;
//    fn claim_profile(&self, home: ProfileId, profile: ProfileId)
//        -> Box< Future<Item=Rc<MyProfile>, Error=Error> >;
}



pub struct SignerFactory
{
    // TODO this should also support HW wallets
    signers: HashMap<ProfileId, Rc<Signer>>,
}

impl SignerFactory
{
    pub fn new(signers: HashMap<ProfileId, Rc<Signer>>) -> Self
        { Self{signers} }

    pub fn signer(&self, profile_id: &ProfileId) -> Option<Rc<Signer>>
        { self.signers.get(profile_id).map( |s| s.clone() ) }
}



pub struct MyProfileFactory
{
    signer_factory: Rc<SignerFactory>,
    profile_repo:   Rc<SimpleProfileRepo>, // TODO use Rc<ProfileRepo> after TheButton testing
    home_connector: Rc<HomeConnector>,
    handle:         reactor::Handle,
    cache:          Rc<RefCell< HashMap<ProfileId, Rc<MyProfile>> >>,
}


// TODO maybe this should be merged into AdminSessionImpl, the only thing it does is caching
impl MyProfileFactory
{
    //pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<ProfileRepo>, home_connector: Rc<HomeConnector>)
    pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<SimpleProfileRepo>,
               home_connector: Rc<HomeConnector>, handle: reactor::Handle) -> Self
        { Self{ signer_factory, profile_repo, home_connector, handle, cache: Default::default() } }

    pub fn create(&self, own_profile: OwnProfile) -> Result<Rc<MyProfile>,Error>
    {
        let profile_id = own_profile.profile.id.clone();
        if let Some(ref my_profile_rc) = self.cache.borrow().get(&profile_id)
            { return Ok( Rc::clone(my_profile_rc) ) }

        debug!("Creating new profile wrapper for profile {}", profile_id);
        self.signer_factory.signer(&profile_id)
            .map( |signer| {
                let result = MyProfileImpl::new(own_profile, signer, self.profile_repo.clone(),
                    self.home_connector.clone(), self.handle.clone() );
                let result_rc = Rc::new(result) as Rc<MyProfile>;
                // TODO this allows initiating several fill attempts in parallel
                //      until first one succeeds, last one wins by overwriting.
                //      Is this acceptable?
                self.cache.borrow_mut().insert(profile_id, result_rc.clone() );
                result_rc
            } )
            .ok_or( ErrorKind::FailedToAuthorize.into() )
    }
}



pub struct AdminSessionImpl
{
//    keyvault:   Rc<KeyVault>,
//    pathmap:    Rc<Bip32PathMapper>,
//    accessman:  Rc<AccessManager>,
    ui:             Rc<UserInterface>,
    my_profile_ids: Rc<HashSet<ProfileId>>,
    profile_store:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
    profile_factory:Rc<MyProfileFactory>,
//    handle:         reactor::Handle,
}


impl AdminSessionImpl
{
    pub fn new(ui: Rc<UserInterface>, my_profile_ids: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               profile_factory: Rc<MyProfileFactory>) //, handle: reactor::Handle)
        -> Rc<AdminSession>
    {
        let this = Self{ ui, profile_store, my_profile_ids, profile_factory }; //, handle };
        Rc::new(this)
    }
}


impl AdminSession for AdminSessionImpl
{
    fn profiles(&self) -> Box< Future<Item=Vec<Rc<MyProfile>>, Error=Error> >
    {
        // TODO consider delegating implementation to profile(id)
        let store = self.profile_store.clone();
        let prof_factory = self.profile_factory.clone();
        let profile_futs = self.my_profile_ids.iter()
            .map( |prof_id| {
                let prof_factory = prof_factory.clone();
                store.borrow().get( prof_id.to_owned() )
                    .map_err( |e| e.context(ErrorKind::FailedToLoadProfile).into() )
                    .and_then( move |own_profile| prof_factory.create(own_profile) )
            } )
            .collect::<Vec<_>>();
        let profiles_fut = future::join_all(profile_futs);
        Box::new(profiles_fut)
    }

    fn profile(&self, id: ProfileId) -> Box< Future<Item=Rc<MyProfile>, Error=Error> >
    {
        let profile_factory = self.profile_factory.clone();
        let fut = self.profile_store.borrow().get( id.to_owned() )
            .map_err( |e| e.context(ErrorKind::FailedToLoadProfile).into() )
            .and_then( move |own_profile| profile_factory.create(own_profile) );
        Box::new(fut)
    }

    fn create_profile(&self) -> Box< Future<Item=Rc<MyProfile>, Error=Error> >
    {
        unimplemented!()
    }

//    fn claim_profile(&self, home_id: ProfileId, profile: ProfileId) ->
//        Box< Future<Item=Rc<MyProfile>, Error=Error> >
//    {
//        let claim_fut = self.connect_home(&home_id)
//            .map_err(|err| err.context(ErrorKind::ConnectionToHomeFailed).into())
//            .and_then( move |home| {
//                home.claim(profile)
//                    .map_err(|err| err.context(ErrorKind::FailedToClaimProfile).into())
//            });
//        Box::new(claim_fut)
//    }

    fn remove_profile(&self, _profile: &ProfileId)
        -> Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }
}



pub struct ConnectService
{
//    keyvault:       Rc<KeyVault>,
//    pathmap:        Rc<Bip32PathMapper>,
//    accessman:      Rc<AccessManager>,
    ui:             Rc<UserInterface>,
    my_profile_ids: Rc<HashSet<ProfileId>>,
    profile_store:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
    profile_factory:Rc<MyProfileFactory>,
//    handle:         reactor::Handle,
}


impl ConnectService
{
    pub fn new(ui: Rc<UserInterface>, my_profile_ids: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               profile_factory: Rc<MyProfileFactory>) //, handle: &reactor::Handle)
        -> Self
    { Self{ ui, my_profile_ids: my_profile_ids, profile_store, profile_factory: profile_factory } } //, handle: handle.clone() } }


    pub fn admin_session(&self, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<AdminSession>, Error=Error> >
    {
        let adm = AdminSessionImpl::new(self.ui.clone(), self.my_profile_ids.clone(),
            self.profile_store.clone(), self.profile_factory.clone() ); //, self.handle.clone() );

        Box::new( Ok(adm).into_future() )
    }
}


impl DAppEndpoint for ConnectService
{
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >
    {
        let app = app.to_owned();
        let profile_store = self.profile_store.clone();
        let profile_factory = self.profile_factory.clone();
        let fut = self.ui.select_profile()
            .and_then( move |profile_id| {
                let store = profile_store.borrow();
                store.get(profile_id)
                    .map_err( |err| err.context(ErrorKind::FailedToLoadProfile).into() )
            } )
            .and_then( move |own_profile| profile_factory.create(own_profile) )
            .map( move |my_profile| DAppConnect::new(my_profile, app) )
            .map_err( |err| { debug!("Failed to initialize dapp session: {:?}", err); err } );
        Box::new(fut)
    }
}



pub struct DummyUserInterface
{
    my_profiles: Rc<HashSet<ProfileId>>,
}

impl DummyUserInterface
{
    pub fn new(my_profiles: Rc<HashSet<ProfileId>>) -> Self
        { Self{my_profiles} }
}

impl UserInterface for DummyUserInterface
{
    fn initialize(&self) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }

    fn confirm_dappaction(&self, _action: &DAppAction) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }

    fn confirm_pairing(&self, _request: &RelationHalfProof) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }

    fn notify_pairing(&self, _response: &RelationProof) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }

    fn select_profile(&self) -> Box< Future<Item=ProfileId, Error=Error> >
    {
        let first_profile_res = self.my_profiles.iter().cloned().nth(0)
            .ok_or( Error::from(ErrorKind::FailedToAuthorize) );
        Box::new( first_profile_res.into_future() )
    }

    fn manage_profiles(&self) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }
}