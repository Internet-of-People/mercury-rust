use std::cell::RefCell;
use std::rc::Rc;
use std::error::Error;

use futures::{future, sync, Future, Sink};
use futures::sync::mpsc;
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use mercury_storage::error::StorageError;



pub struct HomeServer
{
    handle:             reactor::Handle,
    validator:          Rc<Validator>,
    public_profile_dht: Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
    hosted_profile_db:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
}

impl HomeServer
{
    pub fn new(handle: &reactor::Handle,
               validator: Rc<Validator>,
               public_dht: Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
               private_db: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>) -> Self
    { Self{ handle: handle.clone(), validator: validator,
            public_profile_dht: public_dht, hosted_profile_db: private_db } }
}



pub struct ClientContext
{
    signer:             Rc<Signer>,
    client_pub_key:     PublicKey,
    client_profile_id:  ProfileId,
    //client_profile: Profile,
}

impl ClientContext
{
    pub fn new(signer: Rc<Signer>, client_pub_key: PublicKey, client_profile_id: ProfileId) -> Self // client_profile: Profile) -> Self
        { Self{ signer: signer, client_pub_key: client_pub_key, client_profile_id: client_profile_id } } //  client_profile: client_profile } }
}

impl PeerContext for ClientContext
{
    fn my_signer(&self) -> &Signer { &*self.signer }
    fn peer_pubkey(&self) -> &PublicKey { &self.client_pub_key }
    fn peer_id(&self) -> &ProfileId { &self.client_profile_id }
}



pub struct HomeConnectionServer
{
    server:     Rc<HomeServer>, // TODO consider if we should have a RefCell<> for mutability here
    context:    Rc<PeerContext>,
}



impl HomeConnectionServer
{
    pub fn new(context: Rc<PeerContext>, server: Rc<HomeServer>) -> Result<Self, ErrorToBeSpecified>
    {
        context.validate(&*server.validator)?;

        Ok( Self{ context: context, server: server } )
    }
}



impl ProfileRepo for HomeConnectionServer
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        HomeStream<Profile, String>
    {
        let (send, receive) = mpsc::channel(1);
        receive
    }

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let profile_fut = self.server.public_profile_dht.borrow().get( id.to_owned() )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        Box::new(profile_fut)
    }

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        // TODO parse URL and fetch profile accordingly
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeServer/ProfileRepo.resolve"))) )
    }
}



impl Home for HomeConnectionServer
{
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        if profile != *self.context.peer_id()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Claim() access denied: you authenticated with a different profile".to_owned() ) ) ) }

        let claim_fut = self.server.hosted_profile_db.borrow().get(profile)
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        Box::new(claim_fut)
    }

    // TODO consider how to issue and process invites
    fn register(&self, own_prof: OwnProfile, half_proof: RelationHalfProof, _invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >
    {
        if own_prof.profile.id != *self.context.peer_id()
            { return Box::new( future::err( (own_prof,ErrorToBeSpecified::TODO( "Register() access denied: you authenticated with a different profile".to_owned() )) ) ) }
        if own_prof.profile.pub_key != *self.context.peer_pubkey()
            { return Box::new( future::err( (own_prof,ErrorToBeSpecified::TODO( "Register() access denied: you authenticated with a different public key".to_owned() )) ) ) }

        let own_prof_original = own_prof.clone();
        let error_mapper = |e: StorageError| ( own_prof_original, ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        let error_mapper_clone = error_mapper.clone();

        // TODO we should add our home details with signed RelationProof here into the persona facet's home vector in this profile
        let pub_prof_modified = own_prof.profile.clone();
        let own_prof_modified = own_prof.clone();
        let local_store = self.server.hosted_profile_db.clone();
        let distributed_store = self.server.public_profile_dht.clone();
        let reg_fut = self.server.hosted_profile_db.borrow().get( own_prof.profile.id.clone() )
            .then( |get_res|
            {
                match get_res {
                    Ok(_stored_prof) => Err( ( own_prof, ErrorToBeSpecified::TODO( "Register() rejected: this profile is already hosted".to_owned() ) ) ),
                    // TODO only errors like NotFound should be accepted here but other (e.g. I/O) errors should be delegated
                    Err(e) => Ok( () ),
                }
            } )
            // NOTE Block with "return" is needed, see https://stackoverflow.com/questions/50391668/running-asynchronous-mutable-operations-with-rust-futures
            .and_then( move |_| { // Store public profile parts in distributed storage (e.g. DHT)
                return distributed_store.borrow_mut().set( pub_prof_modified.id.clone(), pub_prof_modified )
                    .map_err(error_mapper_clone ); } )
            .and_then( move |_| { // Store private profile info in local storage only (e.g. SQL)
                return local_store.borrow_mut().set( own_prof_modified.profile.id.clone(), own_prof_modified.clone() )
                    .map( |_| own_prof_modified )
                    .map_err(error_mapper); } );

        Box::new(reg_fut)
    }


    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        if profile != *self.context.peer_id()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Login() access denied: you authenticated with a different profile".to_owned() ) ) ) }

        let val_fut = self.server.hosted_profile_db.borrow().get(profile)
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
            .map( {
                let context_clone = self.context.clone();
                let server_clone = self.server.clone();
                move |_own_profile| Box::new( HomeSessionServer::new(context_clone, server_clone) ) as Box<HomeSession>
            } );

        Box::new(val_fut)
    }


    // NOTE acceptor must have this server as its home
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO check if targeted profile id is hosted on this machine
        //      and delegate the proof to its buffer (if offline) or sink (if logged in)
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.pair_request "))) )
    }


    // NOTE acceptor must have this server as its home
    fn pair_response(&self, rel: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO check if targeted profile id is hosted on this machine
        //      and delegate the proof to its buffer (if offline) or sink (if logged in)
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.pair_response "))) )
    }

    fn call(&self, app: ApplicationId, call_req: CallRequestDetails) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
    {
        // TODO check if targeted profile id is hosted on this machine
        //      and delegate the call to its buffer (if offline) or sink (if logged in)
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.call "))) )
    }
}



pub struct HomeSessionServer
{
    // TODO consider using Weak<Ptrs> instead of Rc<Ptrs> if a closed Home connection cannot
    //      drop all related session automatically
    context:    Rc<PeerContext>,
    server:     Rc<HomeServer>,
    events:     RefCell<Option< HomeSink<ProfileEvent, String> >>,
//    client_profile: OwnProfile,
//    home: Weak<HomeServer>,
}


impl HomeSessionServer
{
    // TODO consider if validating the context is needed here, e.g. as an assert()
    pub fn new(context: Rc<PeerContext>, server: Rc<HomeServer>) -> Self
        { Self{ context: context, server: server, events: RefCell::new(None) } }
}


impl HomeSession for HomeSessionServer
{
    fn update(&self, own_prof: OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        if own_prof.profile.id != *self.context.peer_id()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Update() access denied: you authenticated with a different profile".to_owned() ) ) ) }
        if own_prof.profile.pub_key != *self.context.peer_pubkey()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Update() access denied: you authenticated with a different public key".to_owned() )) ) }

        let upd_fut = self.server.hosted_profile_db.borrow().get( own_prof.profile.id.clone() )
            // NOTE Block with "return" is needed, see https://stackoverflow.com/questions/50391668/running-asynchronous-mutable-operations-with-rust-futures
            .and_then( {
                let distributed_store = self.server.public_profile_dht.clone();
                let pub_prof = own_prof.profile.clone();
                move |_own_prof_orig| { // Update public profile parts in distributed storage (e.g. DHT)
                    return distributed_store.borrow_mut().set( pub_prof.id.clone(), pub_prof );
                }
            } )
            .and_then( {
                let local_store = self.server.hosted_profile_db.clone();
                move |_| { // Update private profile info in local storage only (e.g. SQL)
                    return local_store.borrow_mut().set( own_prof.profile.id.clone(), own_prof );
                }
            } )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );

        Box::new(upd_fut)
    }

    // TODO is the ID of the new home enough here or do we need the whole profile?
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO close/drop session connection after successful unregister()
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.unregister "))) )
    }


    fn events(&self) -> HomeStream<ProfileEvent, String>
    {
// TODO solve lifetimes if sending an error is really needed here
//        // NOTE consuming the events stream multiple times is likely a client implementation error
//        if let Some(ref mut old_sender) = *self.events.borrow_mut() {
//            self.server.handle.spawn(
//                old_sender.send( Err( "Repeated call of HomeSession::events() detected, this channel will is dropped, using the new one".to_owned() ) )
//                    .map( |_| () )
//                    .map_err( |_| () )
//            );
//        }

        // Overwrite sink even if it wasn't empty so the corresponding stream is closed
        let (sender, receiver) = sync::mpsc::channel(1);
        *self.events.borrow_mut() = Some(sender);
        receiver
    }

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> HomeStream<Box<IncomingCall>, String>
    {
        let (sender, receiver) = sync::mpsc::channel(1);
        receiver
    }

    // TODO remove this after testing
    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >
    {
        println!("Ping received `{}`, sending it back", txt);
        Box::new( future::ok( txt.to_owned() ) )
    }
}
