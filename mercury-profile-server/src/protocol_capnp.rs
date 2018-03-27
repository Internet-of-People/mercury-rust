use std::rc::Rc;

use capnp::capability::Promise;
use mercury_common::mercury_capnp::*;

use super::*;



pub struct HomeDispatcherCapnProto
{
    home: Rc<Home>,
}

impl HomeDispatcherCapnProto
{
    pub fn new(home: Rc<Home>) -> Self
        { Self{ home: home } }
}

impl mercury_capnp::profile_repo::Server for HomeDispatcherCapnProto
{
    fn list(&mut self, params: mercury_capnp::profile_repo::ListParams,
            mut results: mercury_capnp::profile_repo::ListResults,)
        -> Promise<(), ::capnp::Error>
    {
        // TODO properly implement this
        Promise::result( Ok( () ) )
    }


    fn load(&mut self, params: mercury_capnp::profile_repo::LoadParams,
            mut results: mercury_capnp::profile_repo::LoadResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id_capnp = pry!( pry!( params.get() ).get_profile_id() );
        let load_fut = self.home.load( &profile_id_capnp.into() )
            .map( move |profile| results.get().init_profile().fill_from(&profile) )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(load_fut)
    }


    fn resolve(&mut self, params: mercury_capnp::profile_repo::ResolveParams,
               mut results: mercury_capnp::profile_repo::ResolveResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_url = pry!( pry!( params.get() ).get_profile_url() );
        let res_fut = self.home.resolve(profile_url)
            .map( move |profile| results.get().init_profile().fill_from(&profile) )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(res_fut)
    }
}

impl mercury_capnp::home::Server for HomeDispatcherCapnProto
{
    fn claim(&mut self, params: mercury_capnp::home::ClaimParams,
             mut results: mercury_capnp::home::ClaimResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id_capnp = pry!( pry!( params.get() ).get_profile_id() );
        let claim_fut = self.home.claim( profile_id_capnp.into() )
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling
            .map( move |own_profile|
                results.get().init_own_profile().fill_from(&own_profile) );

        Promise::from_future(claim_fut)
    }

    fn register(&mut self, params: mercury_capnp::home::RegisterParams,
                mut results: mercury_capnp::home::RegisterResults,)
        -> Promise<(), ::capnp::Error>
    {
        let own_prof_capnp = pry!( pry!( params.get() ).get_own_profile() );
        let own_prof = pry!( OwnProfile::try_from(own_prof_capnp) );
        // TODO properly pass Option<invitation> instead of None
        let reg_fut = self.home.register(own_prof, None)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ) // TODO proper error handling
            .map( move |own_profile|
                results.get().init_own_profile().fill_from(&own_profile) );
        Promise::from_future(reg_fut)
    }

    fn login(&mut self, params: mercury_capnp::home::LoginParams,
             mut results: mercury_capnp::home::LoginResults,)
        -> Promise<(), ::capnp::Error>
    {
        let profile_id = pry!( pry!( params.get() ).get_profile_id() );
        // TODO profile_id must be used to build session
        let session = mercury_capnp::home_session::ToClient::new( HomeSessionDispatcher::new() )
            .from_server::<::capnp_rpc::Server>();
        results.get().set_session(session);
        Promise::ok( () )
    }
}



pub struct HomeSessionDispatcher {}

impl HomeSessionDispatcher
{
    pub fn new() -> Self { Self{} }
}

impl mercury_capnp::home_session::Server for HomeSessionDispatcher
{
    fn ping(&mut self, params: mercury_capnp::home_session::PingParams<>,
            mut results: mercury_capnp::home_session::PingResults<>)
        -> Promise<(), ::capnp::Error>
    {
        let ping = pry!( pry!( params.get() ).get_txt() );
        results.get().set_pong(ping);
        Promise::ok( () )
    }
}
