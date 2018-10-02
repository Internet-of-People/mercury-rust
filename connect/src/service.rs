use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
//use std::fmt::Display;
use std::rc::Rc;

use failure::Fail; // Backtrace, Context
use futures::prelude::*;
use futures::{future, sync::mpsc};
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::{DAppEndpoint, DAppPermission, DAppSession};
use ::error::*;
use ::profile::{EventStream, HomeConnector, MyProfile, MyProfileImpl};
use ::sdk::DAppConnect;
use ::simple_profile_repo::SimpleProfileRepo;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DeviceAuthorization(Vec<u8>);

// TODO should be used in parsed version as something like Vec<(bool,uint16)>
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Bip32Path(String);



// TODO consider using this own error type for the service instead of the imported connect error type
//#[derive(Debug)]
//pub struct Error {
//    inner: Context<ErrorKind>
//}
//
//#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
//pub enum ErrorKind {
//    #[fail(display="unknown")]
//    Unknown,
//}
//
//impl PartialEq for Error {
//    fn eq(&self, other: &Error) -> bool {
//        self.inner.get_context() == other.inner.get_context()
//    }
//}
//
//impl Fail for Error {
//    fn cause(&self) -> Option<&Fail> {
//        self.inner.cause()
//    }
//    fn backtrace(&self) -> Option<&Backtrace> {
//        self.inner.backtrace()
//    }
//}
//
//impl Display for Error {
//    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
//        Display::fmt(&self.inner, f)
//    }
//}
//
//impl Error {
//    pub fn kind(&self) -> ErrorKind {
//        *self.inner.get_context()
//    }
//}
//
//impl From<ErrorKind> for Error {
//    fn from(kind: ErrorKind) -> Error {
//        Error { inner: Context::new(kind) }
//    }
//}
//
//impl From<Context<ErrorKind>> for Error {
//    fn from(inner: Context<ErrorKind>) -> Error {
//        Error { inner: inner }
//    }
//}



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
    fn profiles(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;
    // fn claim(&self, profile_path: TODO_profileId_or_Bip32PAth?) -> Box< Future<Item=Rc<OwnProfile>, Error=Error> >;
    fn create_profile(&self) -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >;

    // TODO separate these profile-related functions below into a separate trait and give a getter method like
    //      fn profile_admin(&self, profile: &ProfileId) -> Future<ProfileAdminSession>
    fn update_profile(&self, profile: &OwnProfile) -> Box< Future<Item=(), Error=Error> >;
    fn remove_profile(&self, profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;

    fn homes(&self, profile: &ProfileId) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn join_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
    fn leave_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
//    fn home_endpoint_hint(&self, home: &ProfileId, endpoint: multiaddr);
//    fn profile_home_hint(&self, profile: &ProfileId, home: &ProfileId);

    fn relations(&self, profile: &ProfileId) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn initiate_relation(&self, my_profile: &ProfileId, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;
    fn accept_relation(&self, half_proof: &RelationHalfProof) -> Box< Future<Item=(), Error=Error> >;
    fn revoke_relation(&self, profile: &ProfileId, relation: &RelationProof) -> Box< Future<Item=(), Error=Error> >;

    fn events(&self, profile: &ProfileId) -> Box< Future<Item=EventStream, Error=Error>>;
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
    // TODO return to ProfileRepo type after testing and separating service with RPC
    profile_repo:   Rc<SimpleProfileRepo>, // TODO use Rc<ProfileRepo> after TheButton testing
    home_connector: Rc<HomeConnector>,
    handle:         reactor::Handle,
    cache:          Rc<RefCell< HashMap<ProfileId, Rc<MyProfile>> >>,
}


impl MyProfileFactory
{
    //pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<ProfileRepo>, home_connector: Rc<HomeConnector>)
    pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<SimpleProfileRepo>,
               home_connector: Rc<HomeConnector>, handle: reactor::Handle) -> Self
        { Self{ signer_factory, profile_repo, home_connector, handle, cache: Default::default() } }

    pub fn create(&self, profile_id: &ProfileId) -> Option<Rc<MyProfile>>
    {
        if let Some(ref my_profile_rc) = self.cache.borrow().get(profile_id)
            { return Some( Rc::clone(my_profile_rc) ) }

        debug!("Creating new profile wrapper for profile {}", profile_id);
        self.signer_factory.signer(profile_id)
            .map( |signer| {
                let result = MyProfileImpl::new(signer, self.profile_repo.clone(),
                    self.home_connector.clone(), self.handle.clone() );
                let result_rc = Rc::new(result) as Rc<MyProfile>;
                // TODO this allows initiating several fill attempts in parallel
                //      until first one succeeds, last one wins by overwriting.
                //      Is this acceptable?
                self.cache.borrow_mut().insert(profile_id.to_owned(), result_rc.clone() );
                result_rc
            } )
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
    fn profiles(&self)
        -> Box< Future<Item=Vec<OwnProfile>, Error=Error> >
    {
        let store = self.profile_store.clone();
        let profile_futs = self.my_profile_ids.iter()
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

//    fn claim(&self, profile_path: TODO_profileId_or_Bip32PAth?)
//        -> Box< Future<Item=Rc<OwnProfile>, Error=Error> >
//    {
//        unimplemented!()
//    }

    fn update_profile(&self, _profile: &OwnProfile)
        -> Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }

    fn remove_profile(&self, _profile: &ProfileId)
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
        let profile_factory_clone = self.profile_factory.clone();
        let profile_factory_clone2 = self.profile_factory.clone();
        let fut = self.profile_store.borrow().get( profile.to_owned() )
            .map_err( |e| e.context(ErrorKind::Unknown).into() )
            .and_then( move |own_profile| {
                match profile_factory_clone.create(&profileid_clone) {
                    Some(my_profile) => Ok( (my_profile, own_profile) ),
                    None => Err(Error::from(ErrorKind::Unknown)),
                }
            } )
            .and_then( |(my_profile, own_profile)| {
                debug!("Connecting to home server and registering my profile there");
                Box::new(my_profile.register(homeid_clone, own_profile, None)
                    .map_err( |(_ownprof, e)| e.context(ErrorKind::Unknown).into()) ) // as Box<Future<Item=OwnProfile, Error=_>>
            } )
            .and_then( move |own_profile| {
                debug!("Saving private profile data to local device storage");
                let mut profiles = profile_store_clone.borrow_mut();
                profiles.set( profileid_clone2, own_profile.clone() )
// TODO REMOVE THIS AFTER TESTING
                    .and_then( move |()| profile_factory_clone2.profile_repo.insert( own_profile.profile.clone() )
                        .map( |()| own_profile ) )
                    .inspect( |_| { let remove_this_after_testing = true; } )
                    .map_err( |e| e.context( ErrorKind::Unknown).into() )
            } );
        Box::new(fut)
    }


    fn leave_home(&self, _profile: &ProfileId, _home: &ProfileId)
        -> Box< Future<Item=OwnProfile, Error=Error> >
    {
        unimplemented!()
    }


    fn relations(&self, profile_id: &ProfileId) ->
        Box< Future<Item=Vec<RelationProof>, Error=Error> >
    {
        let my_profile = match self.profile_factory.create(profile_id) {
            Some(profile) => profile,
            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        let fut = my_profile.relations()
            .map_err( |e| e.context(ErrorKind::Unknown).into() );
        Box::new(fut)
    }


    fn initiate_relation(&self, my_profile_id: &ProfileId, with_profile: &ProfileId) ->
        Box< Future<Item=(), Error=Error> >
    {
        let my_profile = match self.profile_factory.create(my_profile_id) {
            Some(profile) => profile,
            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        let init_fut = my_profile
            .send_pairing_request(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN, &with_profile, None)
            .map_err( |err| err.context(ErrorKind::Unknown).into() );
        Box::new(init_fut)
    }


    fn accept_relation(&self, half_proof: &RelationHalfProof) ->
        Box< Future<Item=(), Error=Error> >
    {
        let my_profile = match self.profile_factory.create(&half_proof.peer_id) {
            Some(profile) => profile,
            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        let proof = match RelationProof::sign_remaining_half( &half_proof, my_profile.signer() ) {
            Ok(proof) => proof,
            Err(e) => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        let init_fut = my_profile.send_pairing_response( proof.clone() )
            .map_err( |err| err.context(ErrorKind::Unknown).into() )
            .and_then( move |()| my_profile.on_new_relation(proof)
                .map_err( |()| ErrorKind::Unknown.into() ) );
        Box::new(init_fut)
    }

    fn revoke_relation(&self, _profile: &ProfileId, _relation: &RelationProof) ->
        Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }


    fn events(&self, profile: &ProfileId) -> Box< Future<Item=EventStream, Error=Error>>
    {
        let my_profile = match self.profile_factory.create(profile) {
            Some(profile) => profile,
            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        let fut = my_profile.login()
            .map( |my_session| {
                // TODO consider if this is right here, or maybe this service (after splitted by profile)
                //      should own and dispatch the original event stream and handle listeners
                let (listener, events) = mpsc::channel(CHANNEL_CAPACITY);
                my_session.add_listener(listener);
                events
            } )
            .map_err( |err| err.context(ErrorKind::Unknown).into() );;
        Box::new(fut)
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
        let profile_factory = self.profile_factory.clone();
        let fut = self.ui.select_profile()
            .and_then( move |profile_id| profile_factory.create(&profile_id)
                .ok_or( Error::from(ErrorKind::Unknown) ) )
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
            .ok_or( Error::from(ErrorKind::Unknown) );
        Box::new( first_profile_res.into_future() )
    }

    fn manage_profiles(&self) -> Box< Future<Item=(), Error=Error> >
    {
        Box::new( Ok( () ).into_future() )
    }
}