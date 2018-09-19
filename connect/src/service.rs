use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::rc::Rc;

use failure::{Backtrace, Context, Fail};
use futures::prelude::*;
use futures::future;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::{DAppSession, Relation};
use ::client::{HomeConnector, ProfileGateway, ProfileGatewayImpl};
use ::sdk::DAppConnect;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DeviceAuthorization(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppPermission(Vec<u8>);

// TODO should be used in parsed version as something like Vec<(bool,uint16)>
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Bip32Path(String);



#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display="unknown")]
    Unknown,
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        self.inner.get_context() == other.inner.get_context()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner: inner }
    }
}



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
    fn confirm(&self, action: &DAppAction)
        -> Box< Future<Item=Signature, Error=Error> >;

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



pub trait ConnectService
{
    // NOTE this implicitly asks for user interaction (through UI) selecting a profile to be used with the app
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >;

    // TODO The Settings app is not really a dApp but a privilegized system app, might use different authorization
    // TODO assigning a UI is more like an initialization process than an Rpc endpoint, reconsider the ui argument
    fn admin_endpoint(&self, // ui: Rc<UserInterface>,
                      authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<AdminEndpoint>, Error=Error> >;
}


pub trait AdminEndpoint
{
    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;
    fn create_profile(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;
    fn update_profile(&self, profile: &OwnProfile) -> Box< Future<Item=(), Error=Error> >;
    fn remove_profile(&self, profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;

    fn homes(&self, profile: &ProfileId) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn join_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
    fn leave_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;

    fn relations(&self) -> Box< Future<Item=Vec<Relation>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn initiate_relation(&self, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;
    fn accept_relation(&self, half_proof: &RelationHalfProof) -> Box< Future<Item=(), Error=Error> >;
    fn revoke_relation(&self, relation: &RelationProof) -> Box< Future<Item=(), Error=Error> >;
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



pub struct ProfileGatewayFactory
{
    signer_factory: Rc<SignerFactory>,
    // TODO return to ProfileRepo type after testing and separating service with RPC
    profile_repo:   Rc<::simple_profile_repo::SimpleProfileRepo>, // Rc<ProfileRepo>,
    home_connector: Rc<HomeConnector>,
}


impl ProfileGatewayFactory
{
    //pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<ProfileRepo>, home_connector: Rc<HomeConnector>)
    pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<::simple_profile_repo::SimpleProfileRepo>, home_connector: Rc<HomeConnector>)
        -> Self { Self{signer_factory, profile_repo, home_connector} }

    pub fn gateway(&self, profile_id: &ProfileId) -> Option<Rc<ProfileGateway>>
    {
        let repo = self.profile_repo.clone();
        let conn = self.home_connector.clone();
        debug!("Creating new gateway for profile {}", profile_id);
        self.signer_factory.signer(profile_id)
            .map( move |signer|
                Rc::new( ProfileGatewayImpl::new(signer, repo, conn) ) as Rc<ProfileGateway> )
    }
}



pub struct SettingsImpl
{
//    keyvault:   Rc<KeyVault>,
//    pathmap:    Rc<Bip32PathMapper>,
//    accessman:  Rc<AccessManager>,
//    ui:         Rc<UserInterface>,
    my_profiles:    Rc<HashSet<ProfileId>>,
    profile_store:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
    gateways:       Rc<ProfileGatewayFactory>,
}


impl SettingsImpl
{
    pub fn new(my_profiles: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               gateways: Rc<ProfileGatewayFactory>) -> Self
    { Self{my_profiles, gateways, profile_store} }
}


impl AdminEndpoint for SettingsImpl
{
    fn profiles(&self)
        -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >
    {
        let store = self.profile_store.clone();
        let profile_futs = self.my_profiles.iter()
            .map( move |x| store.borrow().get( x.to_owned() )
                .map_err( |e| e.context(ErrorKind::Unknown).into() ) )
            .collect::<Vec<_>>();
        let profiles_fut = future::join_all(profile_futs);
        Box::new(profiles_fut)
    }

    fn create_profile(&self)
        -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >
    {
        unimplemented!()
    }

    fn update_profile(&self, profile: &OwnProfile)
        -> Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }

    fn remove_profile(&self, profile: &ProfileId)
        -> Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }


    fn homes(&self, profile_id: &ProfileId)
        -> Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        let fut = self.profile_store.borrow().get( profile_id.to_owned() )
            .map_err( |e| e.context(ErrorKind::Unknown).into() )
            .and_then( |ownprofile| match ownprofile.profile.facet {
                ProfileFacet::Persona(persona) => Ok(persona.homes),
                _ => Err( Error::from(ErrorKind::Unknown) )
            }.into_future() );
        Box::new(fut)
    }

    fn join_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=OwnProfile, Error=Error> >
    {
        debug!("Initializing home registration");
        let homeid_clone = home.to_owned();
        let profileid_clone = profile.to_owned();
        let profileid_clone2 = profile.to_owned();
        let profile_store_clone = self.profile_store.clone();
        let gateways_clone = self.gateways.clone();
        let gateways_clone2 = self.gateways.clone();
        let fut = self.profile_store.borrow().get( profile.to_owned() )
            .map_err( |e| e.context(ErrorKind::Unknown).into() )
            .and_then( move |own_profile| {
                match gateways_clone.gateway(&profileid_clone) {
                    Some(gateway) => Ok( (gateway, own_profile) ),
                    None => Err(Error::from(ErrorKind::Unknown)),
                }
            } )
            .and_then( |(gateway,own_profile)| {
                debug!("Connecting to home server and registering my profile there");
                Box::new(gateway.register(homeid_clone, own_profile, None)
                    .map_err( |(_ownprof, e)| e.context(ErrorKind::Unknown).into()) ) // as Box<Future<Item=OwnProfile, Error=_>>
            } )
            .and_then( move |own_profile| {
                debug!("Saving private profile data to local device storage");
                let mut profiles = profile_store_clone.borrow_mut();
                profiles.set( profileid_clone2, own_profile.clone() )
                    // TODO REMOVE THIS AFTER TESTING
                    .map( move |()| { gateways_clone2.profile_repo.insert( own_profile.profile.clone() ); own_profile } )
                    .map_err( |e| e.context(ErrorKind::Unknown).into() )
            } );
        Box::new(fut)
    }

    fn leave_home(&self, profile: &ProfileId, home: &ProfileId)
        -> Box< Future<Item=OwnProfile, Error=Error> >
    {
        unimplemented!()
    }


    fn relations(&self) ->
        Box< Future<Item=Vec<Relation>, Error=Error> >
    {
        unimplemented!()
    }

    fn initiate_relation(&self, with_profile: &ProfileId) ->
        Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }

    fn accept_relation(&self, half_proof: &RelationHalfProof) ->
        Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }

    fn revoke_relation(&self, relation: &RelationProof) ->
        Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }
}



pub struct ServiceImpl
{
//    keyvault:       Rc<KeyVault>,
//    pathmap:        Rc<Bip32PathMapper>,
//    accessman:      Rc<AccessManager>,
    ui:             Rc<UserInterface>,
    my_profiles:    Rc<HashSet<ProfileId>>,
    profile_store:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
    gateways:       Rc<ProfileGatewayFactory>,
    handle:         reactor::Handle,
}


impl ServiceImpl
{
    pub fn new(ui: Rc<UserInterface>, my_profiles: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               gateways: Rc<ProfileGatewayFactory>, handle: &reactor::Handle) -> Self
    { Self{ ui, my_profiles, profile_store, gateways, handle: handle.clone() } }
}


impl ConnectService for ServiceImpl
{
    fn dapp_session(&self, app: &ApplicationId, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<DAppSession>, Error=Error> >
    {
        let app = app.to_owned();
        let handle = self.handle.clone();
        let gateways = self.gateways.clone();
        let fut = self.ui.select_profile()
            .and_then( move |profile_id| gateways.gateway(&profile_id)
                .ok_or( Error::from(ErrorKind::Unknown) ) )
            .map( move |gateway| Rc::new( DAppConnect::new(gateway, &app, &handle) ) as Rc<DAppSession> )
            .map_err( |err| { debug!("Failed to initialize dapp session: {:?}", err); err } );
        Box::new(fut)
    }

    fn admin_endpoint(&self, // ui: Rc<UserInterface>,
                      authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<AdminEndpoint>, Error=Error> >
    {
        let settings = SettingsImpl::new( self.my_profiles.clone(),
            self.profile_store.clone(), self.gateways.clone() );
        Box::new( Ok( Rc::new(settings) as Rc<AdminEndpoint> ).into_future() )
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

    fn confirm(&self, action: &DAppAction) -> Box< Future<Item=Signature, Error=Error> >
    {
        Box::new( Ok( Signature( Vec::new() ) ).into_future() )
    }

    fn select_profile(&self) -> Box< Future<Item=ProfileId, Error=Error> >
    {
        let first_profile_res = self.my_profiles.iter().cloned().nth(0)
            .ok_or( Error::from(ErrorKind::Unknown) );
        Box::new( first_profile_res.into_future() )
    }

    fn manage_profiles(&self) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }
}