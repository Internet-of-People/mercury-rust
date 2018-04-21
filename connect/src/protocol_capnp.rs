use capnp::capability::Promise;
use futures::{Future, Sink};
use futures::sync::mpsc::{self, Sender};
use tokio_core::reactor;
use tokio_core::net::TcpStream;

use mercury_home_protocol::mercury_capnp;
use mercury_capnp::*;

use super::*;

use futures::Stream;


pub struct HomeClientCapnProto
{
    context: Box<PeerContext>,
    repo:    profile_repo::Client,
    home:    home::Client,
    handle:  reactor::Handle,
}


impl HomeClientCapnProto
{
    pub fn new<R,W>(reader: R, writer: W,
               context: Box<PeerContext>, handle: reactor::Handle) -> Self
        where R: std::io::Read + 'static,
              W: std::io::Write + 'static
    {
        println!("Initializing Cap'n'Proto");

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


    pub fn new_tcp(tcp_stream: TcpStream, context: Box<PeerContext>, handle: reactor::Handle) -> Self
    {
        use tokio_io::AsyncRead;

        tcp_stream.set_nodelay(true).unwrap();
        let (reader, writer) = tcp_stream.split();
        HomeClientCapnProto::new(reader, writer, context, handle)
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
        HomeStream<Profile, String>
    {
        // TODO properly implement this
        let (send, recv) = mpsc::channel(1);
        recv
        //Box::new( recv.map_err( |_| "Failed but why? TODO".to_owned() ) ) // TODO
    }


    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let mut request = self.repo.load_request();
        request.get().set_profile_id( id.into() );

        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                let profile_capnp = pry!( pry!( resp.get() ).get_profile() );
                let profile = Profile::try_from(profile_capnp);
                Promise::result(profile)
            } )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.load ")) );

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
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.resolve ")) );

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
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.claim ")) );;

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
            .map_err( move  |e| (own_profile, ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.register "))) );;

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
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.login ")) );;

        Box::new(resp_fut)
    }


    // NOTE acceptor must have this server as its home
    fn pair_request(&self, half_proof: RelationHalfProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.pair_request_request();
        request.get().init_half_proof().fill_from(&half_proof);

        let resp_fut = request.send().promise
            .map(  |resp| () )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.pair_request ")) );;

        Box::new(resp_fut)
    }


    // NOTE acceptor must have this server as its home
    fn pair_response(&self, relation_proof: RelationProof) ->
        Box< Future<Item=(), Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.pair_response_request();
        request.get().init_relation().fill_from(&relation_proof);

        let resp_fut = request.send().promise
            .map(  |resp| () )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.pair_response ")) );

        Box::new(resp_fut)
    }


    fn call(&self, rel: RelationProof, app: ApplicationId, init_payload: AppMessageFrame,
            to_caller: Option<AppMsgSink>) ->
        Box< Future<Item=Option<AppMsgSink>, Error=ErrorToBeSpecified> >
    {
        let mut request = self.home.call_request();
        request.get().init_relation().fill_from(&rel);
        request.get().set_app( (&app).into() );
        request.get().set_init_payload( (&init_payload).into() );

        if let Some(send) = to_caller
        {
            let to_caller_dispatch = mercury_capnp::AppMessageDispatcherCapnProto::new(send);
            let to_caller_capnp = mercury_capnp::app_message_listener::ToClient::new(to_caller_dispatch)
                .from_server::<::capnp_rpc::Server>();
            request.get().set_to_caller(to_caller_capnp);
        }

        let handle_clone = self.handle.clone();
        let resp_fut = request.send().promise
            .and_then( |resp| resp.get()
                .map( |res| res.get_to_callee()
                    .map( |to_callee_capnp| mercury_capnp::fwd_appmsg(to_callee_capnp, handle_clone) )
                    .ok()
                )
            )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeClientCapnProto.call ")) );

        Box::new(resp_fut)
    }
}



struct ProfileEventDispatcherCapnProto
{
    sender: Sender< Result<ProfileEvent, String> >,
}

impl ProfileEventDispatcherCapnProto
{
    fn new(sender: Sender< Result<ProfileEvent, String> >) -> Self
        { Self{ sender: sender } }
}


impl mercury_capnp::profile_event_listener::Server for ProfileEventDispatcherCapnProto
{
    fn receive(&mut self, params: mercury_capnp::profile_event_listener::ReceiveParams,
                 results: mercury_capnp::profile_event_listener::ReceiveResults,)
        -> Promise<(), ::capnp::Error>
    {
        let event_capnp = pry!( pry!( params.get() ).get_event() );
        let event = pry!( ProfileEvent::try_from(event_capnp) );
        let recv_fut = self.sender.clone().send( Ok(event) )
            .map(  |sink| () )
            .map_err( |e| ::capnp::Error::failed( format!("Failed to send event: {}",e ) ) );
        Promise::from_future(recv_fut)
    }


    fn error(&mut self, params: mercury_capnp::profile_event_listener::ErrorParams,
              results: mercury_capnp::profile_event_listener::ErrorResults,)
        -> Promise<(), ::capnp::Error>
    {
        let error = pry!( pry!( params.get() ).get_error() ).into();
        let recv_fut = self.sender.clone().send( Err(error) )
            .map(  |sink| () )
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
            .map(  |resp| () )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeSessionClientCapnProto.update ")) );

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
            .map(  |resp| () )
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeSessionClientCapnproto.unregister ")) );

        Box::new(resp_fut)
    }


    fn events(&self) -> Box< HomeStream<ProfileEvent, String> >
    {
        let (send, recv) = mpsc::channel(1);
        let listener = ProfileEventDispatcherCapnProto::new( send.clone() );
        // TODO consider how to drop/unregister this object from capnp if the stream is dropped
        let listener_capnp = mercury_capnp::profile_event_listener::ToClient::new(listener)
            .from_server::<::capnp_rpc::Server>();

        let mut request = self.session.events_request();
        request.get().set_event_listener(listener_capnp);

        self.handle.spawn(
            request.send().promise
                .map(  |resp| () )
                .or_else( move |e|
                    send.send( Err( format!("Events delegation failed: {}", e) ) )
                        .map(  |sink| () )
                        // TODO what to do if failed to send error?
                        .map_err(  |err| () ) )
        );

        Box::new(recv)
    }


    fn checkin_app(&self, app: &ApplicationId) ->
        Box< HomeStream<Call, String> >
    {
        let (send, recv) = mpsc::channel(1);
        Box::new(recv)
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
            .map_err(  |e| ErrorToBeSpecified::TODO(String::from("HomeSessionClientCapnProto.ping ")) );

        Box::new(resp_fut)
    }
}


#[cfg(test)]
mod tests
{
    use super::*;
    use tokio_core::net::TcpStream;
    use tokio_core::reactor;


    struct TestSetup
    {
        reactor: reactor::Core,
    }

    impl TestSetup
    {
        fn new() -> Self
        {
            Self{ reactor: reactor::Core::new().unwrap() }
        }
    }


    struct DummySigner
    {
        prof_id: ProfileId,
        pub_key: PublicKey,
    }

    impl Signer for DummySigner
    {
        fn prof_id(&self) -> &ProfileId { &self.prof_id }
        fn pub_key(&self) -> &PublicKey { &self.pub_key }
        fn sign(&self, data: &[u8]) -> Signature { Signature( Vec::new() ) }
    }


    #[test]
    fn temporary_test_capnproto()
    {
        use std::net::ToSocketAddrs;
        use std::time::Duration;
        use super::protocol_capnp::*;

        let mut setup = TestSetup::new();

        let prof_id = ProfileId( "joooozsi".as_bytes().to_owned() );
        let home_id = ProfileId( "HomeSweetHome".as_bytes().to_owned() );
        let signer = Rc::new( DummySigner{ prof_id: prof_id.clone(), pub_key: PublicKey(Vec::new()) } );
        let home_facet = HomeFacet{ addrs: Vec::new(), data: Vec::new() };
        let home_prof = Profile::new( &home_id,
            &PublicKey( "HomePubKey".as_bytes().to_owned() ),
            &[ ProfileFacet::Home(home_facet) ] );
        let home_ctx = Box::new( HomeContext::new(signer, &home_prof) );

        let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
        let handle = setup.reactor.handle();
        let handle2 = setup.reactor.handle();
        let handle3 = setup.reactor.handle();
        let test_fut = TcpStream::connect( &addr, &setup.reactor.handle() )
            .map_err( | e| ErrorToBeSpecified::TODO(String::from("temporaty_test_capnproto fails at connect ")) )
            .and_then( move |tcp_stream|
            {
                let home = HomeClientCapnProto::new_tcp(tcp_stream, home_ctx, handle);
                //home.load(&prof_id)
                home.login(prof_id)
            } )
            .and_then( |session| reactor::Timeout::new( Duration::from_secs(5), &handle2 ).unwrap()
                .map( move |_| session )
                .map_err( |_| ErrorToBeSpecified::TODO(String::from("temporary_test_capnproto fails at session ")) ) )
            .and_then( |session| session.ping("hahoooo") )
            .and_then( |pong|
            {
                println!("Got pong {}", pong);
                reactor::Timeout::new( Duration::from_secs(5), &handle3 ).unwrap()
                    .map( move |_| pong )
                    .map_err( |_| ErrorToBeSpecified::TODO(String::from("temporary_test_capnproto can't play ping-pong ")) )
            } );

        let pong = setup.reactor.run(test_fut);
        println!("Response: {:?}", pong);

        let handle = setup.reactor.handle();
        setup.reactor.run( reactor::Timeout::new( Duration::from_secs(5), &handle ).unwrap() );
        println!("Client shutdown");
    }
}