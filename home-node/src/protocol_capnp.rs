use std::rc::Rc;

use capnp::capability::Promise;
use futures::{Future, Stream};
use tokio_core::net::TcpStream;
use tokio_core::reactor;

use super::*;
use mercury_home_protocol::*;
use mercury_home_protocol::mercury_capnp::*;



pub struct HomeDispatcherCapnProto
{
    home:   Box<Home>,
    handle: reactor::Handle, // TODO is this member needed for streams or can be deleted?
    // TODO probably we should have a SessionFactory here instead of instantiating sessions "manually"
}


impl HomeDispatcherCapnProto
{
    // TODO how to access PeerContext in the Home implementation?
    pub fn dispatch<R,W>(home: Box<Home>, reader: R, writer: W, handle: reactor::Handle)
        where R: std::io::Read  + 'static,
              W: std::io::Write + 'static
    {
        let dispatcher = Self{ home: home, handle: handle.clone() };

        let home_capnp = mercury_capnp::home::ToClient::new(dispatcher)
            .from_server::<::capnp_rpc::Server>();
        let network = capnp_rpc::twoparty::VatNetwork::new( reader, writer,
            capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default() );

        let rpc_system = capnp_rpc::RpcSystem::new( Box::new(network), Some( home_capnp.clone().client ) );

        handle.spawn( rpc_system.map_err( |e| println!("Capnp RPC failed: {}", e) ) );
    }


    pub fn dispatch_tcp(home: Box<Home>, tcp_stream: TcpStream, handle: reactor::Handle)
    {
        use tokio_io::AsyncRead;

        tcp_stream.set_nodelay(true).unwrap();
        let (reader, writer) = tcp_stream.split();
        HomeDispatcherCapnProto::dispatch(home, reader, writer, handle)
    }
}


impl Drop for HomeDispatcherCapnProto
    { fn drop(&mut self) { println!("Home dropped"); } }


impl profile_repo::Server for HomeDispatcherCapnProto
{
    fn list(&mut self, params: profile_repo::ListParams,
            mut results: profile_repo::ListResults,)
        -> Promise<(), ::capnp::Error>
    {
        // TODO properly implement this
        Promise::result( Ok( () ) )
    }


    fn load(&mut self, params: profile_repo::LoadParams,
            mut results: profile_repo::LoadResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id_capnp = pry!( pry!( params.get() ).get_profile_id() );
        let load_fut = self.home.load( &profile_id_capnp.into() )
            .map( move |profile| results.get().init_profile().fill_from(&profile) )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(load_fut)
    }


    fn resolve(&mut self, params: profile_repo::ResolveParams,
               mut results: profile_repo::ResolveResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_url = pry!( pry!( params.get() ).get_profile_url() );
        let res_fut = self.home.resolve(profile_url)
            .map( move |profile| results.get().init_profile().fill_from(&profile) )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(res_fut)
    }
}



impl home::Server for HomeDispatcherCapnProto
{
    fn claim(&mut self, params: home::ClaimParams,
             mut results: home::ClaimResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id_capnp = pry!( pry!( params.get() ).get_profile_id() );
        let claim_fut = self.home.claim( profile_id_capnp.into() )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling
            .map( move |own_profile|
                results.get().init_own_profile().fill_from(&own_profile) );

        Promise::from_future(claim_fut)
    }


    fn register(&mut self, params: home::RegisterParams,
                mut results: home::RegisterResults,)
        -> Promise<(), ::capnp::Error>
    {
        let own_prof_capnp = pry!( pry!( params.get() ).get_own_profile() );
        let own_prof = pry!( OwnProfile::try_from(own_prof_capnp) );

        let inv_capnp_res = pry!( params.get() ).get_invite();
        let invite_opt = inv_capnp_res
            .and_then( |inv_capnp| HomeInvitation::try_from(inv_capnp) )
            .ok();

        let reg_fut = self.home.register(own_prof, invite_opt)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling
            .map( move |own_profile|
                results.get().init_own_profile().fill_from(&own_profile) );

        Promise::from_future(reg_fut)
    }


    fn login(&mut self, params: home::LoginParams,
             mut results: home::LoginResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id = pry!( pry!( params.get() ).get_profile_id() );
        let session_fut = self.home.login( profile_id.into() )
            .map( move |session_impl|
            {
                let session_dispatcher = HomeSessionDispatcherCapnProto::new(session_impl);
                let session = home_session::ToClient::new(session_dispatcher)
                    .from_server::<::capnp_rpc::Server>();
                results.get().set_session(session);
                ()
            } )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(session_fut)
    }


    fn pair_request(&mut self, params: home::PairRequestParams,
                    mut results: home::PairRequestResults,)
        -> Promise<(), ::capnp::Error>
    {
        let half_proof_capnp = pry!( pry!( params.get() ).get_half_proof() );
        let half_proof = pry!( RelationHalfProof::try_from(half_proof_capnp) );

        let pair_req_fut = self.home.pair_request(half_proof)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(pair_req_fut)
    }


    fn pair_response(&mut self, params: home::PairResponseParams,
                     mut results: home::PairResponseResults,)
        -> Promise<(), ::capnp::Error>
    {
        let proof_capnp = pry!( pry!( params.get() ).get_relation_proof() );
        let proof = pry!( RelationProof::try_from(proof_capnp) );

        let pair_resp_fut = self.home.pair_response(proof)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(pair_resp_fut)
    }


// TODO
//    fn call(&mut self, params: home::CallParams,
//            mut results: home::CallResults,)
//            -> Promise<(), ::capnp::Error>
//    {
//        Promise::ok( () )
//    }
}



pub struct HomeSessionDispatcherCapnProto
{
    session: Box<HomeSession>
}

impl HomeSessionDispatcherCapnProto
{
    pub fn new(session: Box<HomeSession>) -> Self
        { Self{ session: session } }
}

// TODO remove after debugging
impl Drop for HomeSessionDispatcherCapnProto
    { fn drop(&mut self) { println!("Session dropped"); } }

impl home_session::Server for HomeSessionDispatcherCapnProto
{
    fn update(&mut self, params: home_session::UpdateParams,
              mut results: home_session::UpdateResults,)
        -> Promise<(), ::capnp::Error>
    {
        let own_profile_capnp = pry!( pry!( params.get() ).get_own_profile() );
        let own_profile = pry!( OwnProfile::try_from(own_profile_capnp) );

        let upd_fut = self.session.update(&own_profile)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(upd_fut)
    }


    fn unregister(&mut self, params: home_session::UnregisterParams,
                   mut results: home_session::UnregisterResults,)
        -> Promise<(), ::capnp::Error>
    {
        let new_home_res_capnp = pry!( params.get() ).get_new_home();
        let new_home_opt = new_home_res_capnp
            .and_then( |new_home_capnp| Profile::try_from(new_home_capnp) )
            .ok();

        let upd_fut = self.session.unregister(new_home_opt)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(upd_fut)
    }


    fn ping(&mut self, params: home_session::PingParams<>,
            mut results: home_session::PingResults<>)
        -> Promise<(), ::capnp::Error>
    {
        let txt = pry!( pry!( params.get() ).get_txt() );
        let ping_fut = self.session.ping(txt)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling
            .map( move |pong| results.get().set_pong(&pong) );
        Promise::from_future(ping_fut)
    }


    fn events(&mut self, params: home_session::EventsParams<>,
              mut results: home_session::EventsResults<>)
        -> Promise<(), ::capnp::Error>
    {
        let callback = pry!( pry!( params.get() ).get_event_listener() );
        let events_fut = self.session.events()
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling;
            .for_each( move |item|
            {
                match item {
                    Ok(event) => {
                        let mut request = callback.receive_request();
                        request.get().init_event().fill_from(&event);
                        let fut = request.send().promise
                            .map( |_resp| () );
                        // TODO .map_err() what to do here in case of an error?
                        Box::new(fut) as Box< Future<Item=(), Error=::capnp::Error> >
                    },
                    Err(err) => {
                        let mut request = callback.error_request();
                        request.get().set_error(&err);
                        let fut = request.send().promise
                            .map( |_resp| () );
                        // TODO .map_err() what to do here in case of an error?
                        Box::new(fut)
                    }
                }
            } );
        Promise::from_future(events_fut)
    }


    fn checkin_app(&mut self, params: home_session::CheckinAppParams<>,
                   mut results: home_session::CheckinAppResults<>)
        -> Promise<(), ::capnp::Error>
    {
        let params = pry!( params.get() );
        let app_id = pry!( params.get_app() );
        let callback = pry!( params.get_call_listener() );

        let events_fut = self.session.checkin_app( &app_id.into() )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling;
            .for_each( move |call|
            {
                let request = callback.receive_request();
                // request.get().set_call(call);
                request.send().promise
                    .map( |_resp| () )
                    // TODO .map_err() what to do here in case of an error?
            } );
        Promise::from_future(events_fut)
    }
}
