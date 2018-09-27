use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
//use std::fmt::Display;
use std::rc::{Rc, Weak};

use failure::Fail; // Backtrace, Context
use futures::prelude::*;
use futures::{future, sync::mpsc};
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_home_protocol::future as fut;
use mercury_storage::async::KeyValueStore;
use ::{DAppEndpoint, DAppPermission, DAppSession, Relation};
use ::profile::{HomeConnector, MyProfile, MyProfileImpl};
use ::error::*;
use ::sdk::DAppConnect;



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
    //      fn profile_admin(&self, profile: &ProfileId) -> Future<ProfileAdminEndpoint>
    fn update_profile(&self, profile: &OwnProfile) -> Box< Future<Item=(), Error=Error> >;
    fn remove_profile(&self, profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;

    fn homes(&self, profile: &ProfileId) -> Box< Future<Item=Vec<RelationProof>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn join_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
    fn leave_home(&self, profile: &ProfileId, home: &ProfileId) -> Box< Future<Item=OwnProfile, Error=Error> >;
//    fn home_endpoint_hint(&self, home: &ProfileId, endpoint: multiaddr);
//    fn profile_home_hint(&self, profile: &ProfileId, home: &ProfileId);

    fn relations(&self, profile: &ProfileId) -> Box< Future<Item=Vec<Relation>, Error=Error> >;
    // TODO we should be able to handle profile URLs and/or home address hints to avoid needing a profile repository to join the first home node
    fn initiate_relation(&self, my_profile: &ProfileId, with_profile: &ProfileId) -> Box< Future<Item=(), Error=Error> >;
    fn accept_relation(&self, half_proof: &RelationHalfProof) -> Box< Future<Item=(), Error=Error> >;
    fn revoke_relation(&self, profile: &ProfileId, relation: &RelationProof) -> Box< Future<Item=(), Error=Error> >;
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
    profile_repo:   Rc<::simple_profile_repo::SimpleProfileRepo>, // TODO use Rc<ProfileRepo> after TheButton testing
    home_connector: Rc<HomeConnector>,
    cache:          Rc<RefCell< HashMap<ProfileId, Rc<MyProfile>> >>,
}


impl MyProfileFactory
{
    //pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<ProfileRepo>, home_connector: Rc<HomeConnector>)
    pub fn new(signer_factory: Rc<SignerFactory>, profile_repo: Rc<::simple_profile_repo::SimpleProfileRepo>, home_connector: Rc<HomeConnector>)
        -> Self { Self{ signer_factory, profile_repo, home_connector, cache: Default::default() } }

    pub fn create(&self, profile_id: &ProfileId) -> Option<Rc<MyProfile>>
    {
        if let Some(ref my_profile_rc) = self.cache.borrow().get(profile_id)
            { return Some( Rc::clone(my_profile_rc) ) }

        debug!("Creating new profile wrapper for profile {}", profile_id);
        self.signer_factory.signer(profile_id)
            .map( |signer| {
                let result = MyProfileImpl::new(signer, self.profile_repo.clone(), self.home_connector.clone() );
                let result_rc = Rc::new(result) as Rc<MyProfile>;
                // TODO this allows initiating several fill attempts in parallel
                //      until first one succeeds, last one wins by overwriting.
                //      Is this acceptable?
                self.cache.borrow_mut().insert(profile_id.to_owned(), result_rc.clone() );
                result_rc
            } )
    }
}



pub type EventSink   = mpsc::Sender<ProfileEvent>;
pub type EventStream = mpsc::Receiver<ProfileEvent>;

pub struct AdminSessionImpl
{
//    keyvault:   Rc<KeyVault>,
//    pathmap:    Rc<Bip32PathMapper>,
//    accessman:  Rc<AccessManager>,
    ui:             Rc<UserInterface>,
    my_profiles:    Rc<HashSet<ProfileId>>,
    profile_store:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
    profile_factory:Rc<MyProfileFactory>,
    // {persona -> profile_event_listeners}
    event_sinks:    Rc<RefCell< HashMap<ProfileId, Rc<RefCell< Vec<EventSink>> >> >>, // TODO can this be simpler?
    handle:         reactor::Handle,
}


impl AdminSessionImpl
{
    pub fn new(ui: Rc<UserInterface>, my_profile_ids: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               profile_factory: Rc<MyProfileFactory>, handle: &reactor::Handle)
        -> Box< Future<Item=Self, Error=Error> >
    {
        let this = Self{ ui, profile_store, my_profiles: my_profile_ids.clone(),
            profile_factory: profile_factory.clone(), handle: handle.to_owned(),
            event_sinks: Default::default(), };

        // Create a stream of profile events for each home
        let event_futs = my_profile_ids.iter().cloned()
            .filter_map( |profile_id| profile_factory.create(&profile_id) )
            .map( |my_profile| my_profile.login()
                .map( move |session| ( my_profile.signer().profile_id().to_owned(), session.events() ) ) )
            .collect::<Vec<_>>();

//        let events_fut = fut::collect_results(event_futs)
//            .map( |mut results| results.drain(..)
//                // TODO how should we handle errors better then just skipping failing homes?
//                //      We should check if we had at least one successful home
//                .filter_map( |res| res.ok() )
//                .map( move |(profile_id, events)| {
//                    let event_sinks = this.event_sinks.clone();
//                    event_sinks.borrow_mut().insert( profile_id, Vec::new() );
//                    handle.spawn(
//                        events.for_each( move |event|
//                            Self::forward_event_safe(listeners.clone(), event ) ) );
//                } )
//                .collect::<Vec<_>>()
//            );

        Box::new( Ok(this).into_future() )
    }


    pub fn add_listener(event_sinks: Rc<RefCell< Vec<EventSink> >>, sink: EventSink)
        { event_sinks.borrow_mut().push(sink) }


    // Notify all registered listeners of an incoming profile event,
    // removing failing (i.e. dropped) listeners from the list
    fn forward_event(mut sinks: Vec<EventSink>, event: ProfileEvent)
        -> Box< Future<Item=Vec<EventSink>, Error=()> >
    {
        // Create tasks (futures) of sending an item to each listener
        let send_futs = sinks.drain(..)
            .map( |sink| sink.send( event.clone() ) );

        // Collect successful senders, drop failing ones
        let fwd_fut = fut::collect_results(send_futs)
            .map( |mut results| results.drain(..)
                .filter_map( |res| res.ok() ).collect() );

        Box::new(fwd_fut)
    }


    // Call forward event with safety measures on: respect a dropped service and remote errors sent by the home
    fn forward_event_safe(event_sinks_weak: Weak<RefCell< Vec<EventSink> >>,
                          event_res: Result<ProfileEvent,String>)
        -> Box< Future<Item=(), Error=()> >
    {
        // Get strong Rc from Weak, stop forwarding if Rc is already dropped
        let event_sinks_rc = match event_sinks_weak.upgrade() {
            Some(sinks) => sinks,
            None => return Box::new( Err(()).into_future() ), // NOTE error only to break for_each, otherwise normal
        };

        // Try unwrapping and forwarding event, stop forwarding if received remote error
        match event_res {
            Ok(event) => {
                let sinks = event_sinks_rc.replace( Vec::new() );
                let fwd_fut = Self::forward_event(sinks, event)
                    .map( move |successful_sinks| {
                        let mut listeners = event_sinks_rc.borrow_mut();
                        listeners.extend(successful_sinks); // Use extend instead of assignment to keep listeners added meanwhile
                    } );
                Box::new(fwd_fut) as Box<Future<Item=(), Error=()>>
            },
            Err(e) => {
                warn!("Remote error listening to profile events, stopping listeners: {}", e);
                Box::new( Err(()).into_future() )
            },
        }
    }


//    // TODO somehow make sure this is called only once and cached if successful
//    fn login_and_forward_events(&self, profile_id: &ProfileId)
//        -> Box< Future<Item=Rc<HomeSession>, Error=Error> >
//    {
//        let gateway = match self.gateways.gateway(profile_id) {
//            Some(gateway) => gateway,
//            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
//        };
//
//        let log_fut = gateway.login()
//            .map_err( |e| e.context(ErrorKind::Unknown).into() )
//            .inspect( {
//                let handle = self.handle.clone();
//                let listeners = Rc::downgrade(&self.event_sinks);
//                move |session| {
//                    debug!("Login was successful, start forwarding profile events to listeners");
//                    handle.spawn(
//                        session.events().for_each( move |event|
//                            Self::forward_event_safe(listeners.clone(), event ) ) );
//                }
//            } );
//        Box::new(log_fut)
//    }
}


impl AdminSession for AdminSessionImpl
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
                    .map_err( |e| e.context( ErrorKind::Unknown).into() )
            } );
        Box::new(fut)
    }


    fn leave_home(&self, _profile: &ProfileId, _home: &ProfileId)
        -> Box< Future<Item=OwnProfile, Error=Error> >
    {
        unimplemented!()
    }


    fn relations(&self, _profile: &ProfileId) ->
        Box< Future<Item=Vec<Relation>, Error=Error> >
    {
        unimplemented!()
    }


    // TODO should the future wait for a proper response to be received?
    fn initiate_relation(&self, my_profile_id: &ProfileId, with_profile: &ProfileId) ->
        Box< Future<Item=(), Error=Error> >
    {
        let my_profile = match self.profile_factory.create(my_profile_id) {
            Some(profile) => profile,
            None => return Box::new( Err( ErrorKind::Unknown.into() ).into_future() ),
        };

        // let event_sinks = self.event_sinks.clone();
        let with_profile = with_profile.to_owned();
//        let proof_fut = self.login_and_forward_events(my_profile)
//            .and_then( move |_session|
//                gateway.pair_request(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN, &with_profile, None)
//                    .map_err( |err| err.context(ErrorKind::Unknown).into() ) )
//            .inspect( |_| debug!("Pairing request sent, expect response as profile event on my home") )
//            .and_then( |()|
//            {
//                let (event_sink, event_stream) = mpsc::channel(CHANNEL_CAPACITY);
//                Self::add_listener(event_sinks, event_sink);
//
//                event_stream.filter_map( move |event|
//                {
//                    debug!("Profile event listener got event");
//                    if let ProfileEvent::PairingResponse(proof) = event {
//                        debug!("Got pairing response, checking peer id: {:?}", proof);
//                        if proof.peer_id( gateway.signer().profile_id() ).is_ok()
//                            { return Some(proof) }
//                    }
//                    return None
//                } )
//                .take(1)
//                .collect()
//                .map_err( |()| {
//                    debug!("Pairing failed");
//                    Error::from(ErrorKind::Unknown) // TODO
//                } )
//            } )
//            .and_then( |mut proofs| {
//                debug!("Got {} matching pairing response: {:?}", proofs.len(), proofs.last());
//                proofs.pop().ok_or( {
//                    debug!("Profile event stream ended without proper response");
//                    ErrorKind::Unknown.into()
//                } )
//            } ); // TODO
//            ;
//        Box::new(proof_fut)
        unimplemented!()
    }


    fn accept_relation(&self, _half_proof: &RelationHalfProof) ->
        Box< Future<Item=(), Error=Error> >
    {
        unimplemented!()
    }

    fn revoke_relation(&self, _profile: &ProfileId, _relation: &RelationProof) ->
        Box< Future<Item=(), Error=Error> >
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
    handle:         reactor::Handle,
}


impl ConnectService
{
    pub fn new(ui: Rc<UserInterface>, my_profile_ids: Rc<HashSet<ProfileId>>,
               profile_store: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
               profile_factory: Rc<MyProfileFactory>, handle: &reactor::Handle) -> Self
        { Self{ ui, my_profile_ids: my_profile_ids, profile_store, profile_factory: profile_factory, handle: handle.clone() } }


    pub fn admin_session(&self, authorization: Option<DAppPermission>)
        -> Box< Future<Item=Rc<AdminSession>, Error=Error> >
    {
        let settings = AdminSessionImpl::new(self.ui.clone(), self.my_profile_ids.clone(),
                                             self.profile_store.clone(), self.profile_factory.clone(), &self.handle );

        Box::new( settings.map( |adm| Rc::new(adm) as Rc<AdminSession> ) )
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