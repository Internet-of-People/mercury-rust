use std::rc::Rc;
use std::error::Error;

use futures::{future, sync, Future};
use futures::sync::mpsc;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;



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
    context:                Box<PeerContext>,
    validator:              Rc<Validator>,
    distributed_storage:    Rc< KeyValueStore<ProfileId, Profile> >,
    local_storage:          Rc< KeyValueStore<ProfileId, OwnProfile> >,
}



impl HomeServer
{
    pub fn new(context:             Box<PeerContext>,
               validator:           Rc<Validator>,
               distributed_storage: Rc< KeyValueStore<ProfileId, Profile> >,
               local_storage:       Rc< KeyValueStore<ProfileId, OwnProfile> > ) -> Self
        { Self { context: context, validator: validator,
                 distributed_storage: distributed_storage, local_storage: local_storage, } }
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
        let profile_fut = self.distributed_storage.get( id.to_owned() )
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
        // TODO consider if this is needed here or can we safely suppose that it's enforced at context creation already
        if let Err(e) = self.context.validate(&*self.validator)
            { return Box::new( future::err(e) ) }

        if profile != *self.context.peer_id()
            { return Box::new( future::err( ErrorToBeSpecified::TODO( "Access denied: you authenticated with a different profile".to_owned() ) ) ) }

        let claim_fut = self.local_storage.get(profile)
            .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) );
        Box::new(claim_fut)
    }

    fn register(&mut self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >
    {
        Box::new( future::err( (own_prof,ErrorToBeSpecified::TODO(String::from("HomeSession.register "))) ) )
    }


    fn login(&self, profile: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        Box::new( future::ok( Box::new( HomeSessionServer{} ) as Box<HomeSession> ) )
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
    // TODO
    // how to access context to get client profileId?
}


impl HomeSessionServer
{
    pub fn new() -> Self
        { Self{} }
}


impl HomeSession for HomeSessionServer
{
    fn update(&self, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        Box::new( future::err(ErrorToBeSpecified::TODO(String::from("HomeSessionServer.update "))) )
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
