use std::cell::RefCell;
use std::rc::Rc;
use std::error::Error;

use futures::{future, sync, Future};
use futures::sync::mpsc;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use mercury_storage::error::StorageError;



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

//    fn validate(&self, validator: Rc<Validator>) -> Result<(),ErrorToBeSpecified>
//    {
//        validator.validate_profile(&client_pub_key, &client_profile_id)
//            .and_then( |valid| if valid { () } else { ErrorToBeSpecified::TODO( "Invalid profile info".to_owned() ) } );
//    }
}



pub struct HomeServer
{
    context:            Rc<PeerContext>,
    validator:          Rc<Validator>,
    public_profile_dht: Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
    hosted_profile_db:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
}



impl HomeServer
{
    pub fn new(context: Rc<PeerContext>, validator: Rc<Validator>,
               public_dht: Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
               private_db: Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>)
        -> Result<Self, ErrorToBeSpecified>
    {
        context.validate(&*validator)?;

        Ok ( Self{ context: context, validator: validator,
                   public_profile_dht: public_dht, hosted_profile_db: private_db } )
    }
}



impl ProfileRepo for HomeServer
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        HomeStream<Profile, String>
    {
        let (send, receive) = mpsc::channel(0);
        receive
    }

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let profile_fut = self.public_profile_dht.borrow().get( id.to_owned() )
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



impl Home for HomeServer
{
    fn claim(&self, profile: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        if profile != *self.context.peer_id()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Claim() access denied: you authenticated with a different profile".to_owned() ) ) ) }

        let claim_fut = self.hosted_profile_db.borrow().get(profile)
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        Box::new(claim_fut)
    }


    // TODO consider how to issue and process invites
    fn register(&mut self, own_prof: OwnProfile, _invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >
    {
        if own_prof.profile.id != *self.context.peer_id()
            { return Box::new( future::err( (own_prof,ErrorToBeSpecified::TODO( "Register() access denied: you authenticated with a different profile".to_owned() )) ) ) }
        if own_prof.profile.pub_key != *self.context.peer_pubkey()
            { return Box::new( future::err( (own_prof,ErrorToBeSpecified::TODO( "Register() access denied: you authenticated with a different public key".to_owned() )) ) ) }

        let own_prof_original = own_prof.clone();
        let error_mapper = |e: StorageError| ( own_prof_original, ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        let error_mapper_clone = error_mapper.clone();

        // TODO should we add our home details here into the persona facet's home vector of this profile?
        let pub_prof_modified = own_prof.profile.clone();
        let own_prof_modified = own_prof.clone();
        let local_store = self.hosted_profile_db.clone();
        let distributed_store = self.public_profile_dht.clone();
        let reg_fut = self.hosted_profile_db.borrow().get( own_prof.profile.id.clone() )
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

        let val_fut = self.hosted_profile_db.borrow().get(profile)
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
            .map( {
                let ctx_clone = self.context.clone();
                let val_clone = self.validator.clone();
                let pub_dht_clone = self.public_profile_dht.clone();
                let priv_db_clone = self.hosted_profile_db.clone();
                move |_own_profile| Box::new( HomeSessionServer::new(ctx_clone, val_clone,
                    pub_dht_clone, priv_db_clone) ) as Box<HomeSession>
            } );

        Box::new(val_fut)
    }


    // NOTE acceptor must have this server as its home
    fn pair_request(&mut self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.pair_request "))) )
    }


    // NOTE acceptor must have this server as its home
    fn pair_response(&mut self, rel: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.pair_response "))) )
    }

    fn call(&self, app: ApplicationId, call_req: CallRequestDetails) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.call "))) )
    }
}



pub struct HomeSessionServer
{
    // TODO consider if we should use Weak<Ptrs> here instead of Rc<Ptrs>
    context:            Rc<PeerContext>,
    validator:          Rc<Validator>,
    public_profile_dht: Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
    hosted_profile_db:  Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>,
//    client_profile: OwnProfile,
//    home: Weak<HomeServer>,
}


impl HomeSessionServer
{
    // TODO consider if validating the context is needed here, e.g. as an assert()
    pub fn new(context: Rc<PeerContext>, validator: Rc<Validator>,
               distributed_db:  Rc<RefCell< KeyValueStore<ProfileId, Profile> >>,
               private_db:      Rc<RefCell< KeyValueStore<ProfileId, OwnProfile> >>) -> Self
        { Self{ context: context, validator: validator,
                public_profile_dht: distributed_db, hosted_profile_db: private_db } }
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

        let upd_fut = self.hosted_profile_db.borrow().get( own_prof.profile.id.clone() )
            // NOTE Block with "return" is needed, see https://stackoverflow.com/questions/50391668/running-asynchronous-mutable-operations-with-rust-futures
            .and_then( {
                let distributed_store = self.public_profile_dht.clone();
                let pub_prof = own_prof.profile.clone();
                move |_own_prof_orig| { // Update public profile parts in distributed storage (e.g. DHT)
                    return distributed_store.borrow_mut().set( pub_prof.id.clone(), pub_prof );
                }
            } )
            .and_then( {
                let local_store = self.hosted_profile_db.clone();
                move |_| { // Update private profile info in local storage only (e.g. SQL)
                    return local_store.borrow_mut().set( own_prof.profile.id.clone(), own_prof );
                }
            } )
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );

        Box::new(upd_fut)
    }

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    // TODO is the ID of the new home enough here or do we need the whole profile?
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        // TODO close/drop session connection after successful unregister()
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.unregister "))) )
    }


    fn events(&self) -> HomeStream<ProfileEvent, String>
    {
        let (sender, receiver) = sync::mpsc::channel(0);
        receiver
    }

    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> HomeStream<Box<IncomingCall>, String>
    {
        let (sender, receiver) = sync::mpsc::channel(0);
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
