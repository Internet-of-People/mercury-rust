use capnp::capability::Promise;
use futures::{Future, Sink};
use futures::sync::mpsc::Sender;
use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tokio_io::AsyncRead;

use mercury_common::mercury_capnp;
use mercury_common::mercury_capnp::*;

use super::*;



pub fn capnp_home(tcp_stream: TcpStream, context: Box<PeerContext>, handle: reactor::Handle) -> Rc<Home>
{
    Rc::new( HomeClientCapnProto::new(tcp_stream, context, handle) )
}



pub struct HomeClientCapnProto
{
    context:Box<PeerContext>,
    repo:   mercury_capnp::profile_repo::Client,
    home:   mercury_capnp::home::Client,
    handle: reactor::Handle,
}


impl HomeClientCapnProto
{
    pub fn new(tcp_stream: TcpStream, context: Box<PeerContext>,
               handle: reactor::Handle) -> Self
    {
        println!("Initializing Cap'n'Proto");
        tcp_stream.set_nodelay(true).unwrap();
        let (reader, writer) = tcp_stream.split();

        // TODO maybe we should set up only single party capnp first
        let rpc_network = Box::new( capnp_rpc::twoparty::VatNetwork::new( reader, writer,
                                                                          capnp_rpc::rpc_twoparty_capnp::Side::Client, Default::default() ) );
        let mut rpc_system = capnp_rpc::RpcSystem::new(rpc_network, None);

        let home: mercury_capnp::home::Client =
            rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);
        let repo: mercury_capnp::profile_repo::Client =
            rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

        handle.spawn( rpc_system.map_err( |e| println!("Capnp RPC failed: {}", e) ) );

        Self{ context: context, home: home, repo: repo, handle: handle }
    }
}



// TODO is this needed here or elsewhere?
//impl PeerContext for HomeClientCapnProto
//{
//    fn my_signer(&self)     -> &Signer          { self.context.my_signer() }
//    fn peer_pubkey(&self)   -> Option<PublicKey>{ self.context.peer_pubkey() }
//    fn peer(&self)          -> Option<Profile>  { self.context.peer() }
//}



impl ProfileRepo for HomeClientCapnProto
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >
    {
        // TODO properly implement this
        let (_send, recv) = futures::sync::mpsc::channel(0);
        Box::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }


    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let mut request = self.repo.load_request();
        request.get().set_profile_id( id.0.as_slice() );

        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                let profile_capnp = pry!( pry!( resp.get() ).get_profile() );
                let profile = Profile::try_from(profile_capnp);
                Promise::result(profile)
            } )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let mut request = self.repo.resolve_request();
        request.get().set_profile_url(url);

        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                let profile_capnp = pry!( pry!( resp.get() ).get_profile() );
                let profile = Profile::try_from(profile_capnp);
                Promise::result(profile)
            } )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }
}



impl Home for HomeClientCapnProto
{
    fn claim(&self, profile_id: ProfileId) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.claim_request();
        request.get().set_profile_id( (&profile_id).into() );

        let resp_fut = request.send().promise
            .and_then( |resp|
                resp.get()
                    .and_then( |res| res.get_own_profile() )
                    .and_then( |own_prof_capnp| OwnProfile::try_from(own_prof_capnp) ) )
            .map_err( |_e| ErrorToBeSpecified::TODO );;

        Box::new(resp_fut)
    }

    fn register(&mut self, own_profile: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=(OwnProfile,ErrorToBeSpecified)> >
    {
        let mut request = self.home.register_request();
        request.get().init_own_profile().fill_from(&own_profile);
        if let Some(inv) = invite
            { request.get().init_invite().fill_from(&inv); }

        let resp_fut = request.send().promise
            .and_then( |resp|
                resp.get()
                    .and_then( |res| res.get_own_profile() )
                    .and_then( |own_prof_capnp| OwnProfile::try_from(own_prof_capnp) ) )
            .map_err( move |_e| (own_profile, ErrorToBeSpecified::TODO) );;

        Box::new(resp_fut)
    }


    fn login(&self, profile_id: ProfileId) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.login_request();
        request.get().set_profile_id( (&profile_id).into() );

        let handle_clone = self.handle.clone();
        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                resp.get()
                    .and_then( |res| res.get_session() )
                    .map( |session_client| Box::new(
                        HomeSessionClientCapnProto::new(session_client, handle_clone) ) as Box<HomeSession> )
            } )
            .map_err( |_e| ErrorToBeSpecified::TODO );;

        Box::new(resp_fut)
    }


    // NOTE acceptor must have this server as its home
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.pair_request_request();
        request.get().init_half_proof().fill_from(&half_proof);

        let resp_fut = request.send().promise
            .map( |resp| () )
            .map_err( |_e| ErrorToBeSpecified::TODO );;

        Box::new(resp_fut)
    }

    // NOTE acceptor must have this server as its home
    fn pair_response(&self, relation_proof: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.pair_response_request();
        request.get().init_relation_proof().fill_from(&relation_proof);

        let resp_fut = request.send().promise
            .map( |resp| () )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }

    fn call(&self, rel: RelationProof, app: ApplicationId, init_payload: AppMessageFrame) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }
}



struct ProfileEventListener
{
    sender: Sender<ProfileEvent>,
}


impl mercury_capnp::profile_event_listener::Server for ProfileEventListener
{
    fn receive(&mut self, params: mercury_capnp::profile_event_listener::ReceiveParams,
                mut results: mercury_capnp::profile_event_listener::ReceiveResults,)
        -> Promise<(), ::capnp::Error>
    {
        let event_capnp = pry!( pry!( params.get() ).get_event() );
        let event = pry!( ProfileEvent::try_from(event_capnp) );
        let recv_fut = self.sender.clone().send(event)
            .map( |_sink| () )
            .map_err( |e| ::capnp::Error::failed( format!("Failed to send event: {}",e ) ) );
        Promise::from_future(recv_fut)
    }
}



pub struct HomeSessionClientCapnProto
{
    session: mercury_capnp::home_session::Client,
    handle:  reactor::Handle,
}

impl HomeSessionClientCapnProto
{
    pub fn new(session: mercury_capnp::home_session::Client, handle: reactor::Handle) -> Self
        { Self{ session: session, handle: handle } }
}

impl HomeSession for HomeSessionClientCapnProto
{
    // TODO consider if we should notify an open session about an updated profile
    // TODO consider if an OwnProfile return value is needed or how to force updating
    //      the currently active profile in all PeerContext/Session/etc instances
    fn update(&self, own_prof: &OwnProfile) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.session.update_request();
        request.get().init_own_profile().fill_from(&own_prof);

        let resp_fut = request.send().promise
            .map( |resp| () )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }


    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    fn unregister(&self, newhome: Option<Profile>) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.session.unregister_request();
        if let Some(new_home_profile) = newhome
            { request.get().init_new_home().fill_from(&new_home_profile); }

        let resp_fut = request.send().promise
            .map( |resp| () )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }


    fn events(&self) -> Box< Stream<Item=ProfileEvent, Error=ErrorToBeSpecified> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        let listener = ProfileEventListener{ sender: send };
        let listener_capnp = mercury_capnp::profile_event_listener::ToClient::new(listener)
            .from_server::<::capnp_rpc::Server>();

        let mut request = self.session.events_request();
        request.get().set_event_listener(listener_capnp);

        // TODO can we avoid handle.spawn() here?
        self.handle.spawn(
            // TODO if not, how to delegate errors to close the stream?
            request.send().promise
                .map( move |_resp| () )
                .map_err( |_e| () )
        );

        Box::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }


    // TODO return not a Stream, but an AppSession struct containing a stream
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< Stream<Item=Call, Error=ErrorToBeSpecified> >
    {
        let (_send, recv) = futures::sync::mpsc::channel(0);
        Box::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }


    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >
    {
        let mut request = self.session.ping_request();
        request.get().set_txt(txt);

        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                resp.get()
                    .and_then( |res| res.get_pong() )
                    .map( |pong| pong.to_owned() )
            } )
            .map_err( |_e| ErrorToBeSpecified::TODO );

        Box::new(resp_fut)
    }
}
