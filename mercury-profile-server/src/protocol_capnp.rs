use std::rc::Rc;

use capnp::capability::Promise;
use mercury_common::mercury_capnp::*;

use super::*;



pub struct HomeDispatcherCapnProto
{
    home: Rc<Home>,
    // TODO probably we should have a SessionFactory here
    //      instead of instantiating sessions "manually"
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


    fn login(&mut self, params: mercury_capnp::home::LoginParams,
             mut results: mercury_capnp::home::LoginResults,)
        -> Promise<(), ::capnp::Error>
    {
        use server::HomeSessionServer;
        let profile_id = pry!( pry!( params.get() ).get_profile_id() );
        // TODO profile_id must be used to build session
        let session_impl = Rc::new( HomeSessionServer::new() );
        let session_dispatcher = HomeSessionDispatcherCapnProto::new(session_impl);
        let session = mercury_capnp::home_session::ToClient::new(session_dispatcher)
            .from_server::<::capnp_rpc::Server>();
        results.get().set_session(session);
        Promise::ok( () )
    }


    fn pair_request(&mut self, params: mercury_capnp::home::PairRequestParams,
                    mut results: mercury_capnp::home::PairRequestResults,)
        -> Promise<(), ::capnp::Error>
    {
        let half_proof_capnp = pry!( pry!( params.get() ).get_half_proof() );
        let half_proof = pry!( RelationHalfProof::try_from(half_proof_capnp) );

        let pair_req_fut = self.home.pair_request(half_proof)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(pair_req_fut)
    }


    fn pair_response(&mut self, params: mercury_capnp::home::PairResponseParams,
                     mut results: mercury_capnp::home::PairResponseResults,)
        -> Promise<(), ::capnp::Error>
    {
        let proof_capnp = pry!( pry!( params.get() ).get_relation_proof() );
        let proof = pry!( RelationProof::try_from(proof_capnp) );

        let pair_resp_fut = self.home.pair_response(proof)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(pair_resp_fut)
    }


// TODO
//    fn call(&mut self, params: mercury_capnp::home::CallParams,
//            mut results: mercury_capnp::home::CallResults,)
//            -> Promise<(), ::capnp::Error>
//    {
//        Promise::ok( () )
//    }
}



pub struct HomeSessionDispatcherCapnProto
{
    session: Rc<HomeSession>
}

impl HomeSessionDispatcherCapnProto
{
    pub fn new(session: Rc<HomeSession>) -> Self
        { Self{ session: session } }
}

impl mercury_capnp::home_session::Server for HomeSessionDispatcherCapnProto
{
    fn update(&mut self, params: mercury_capnp::home_session::UpdateParams,
              mut results: mercury_capnp::home_session::UpdateResults,)
        -> Promise<(), ::capnp::Error>
    {
        let own_profile_capnp = pry!( pry!( params.get() ).get_own_profile() );
        let own_profile = pry!( OwnProfile::try_from(own_profile_capnp) );

        let upd_fut = self.session.update(&own_profile)
            .map_err( |_e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(upd_fut)
    }


    fn unregister(&mut self, params: mercury_capnp::home_session::UnregisterParams,
                   mut results: mercury_capnp::home_session::UnregisterResults,)
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


    fn ping(&mut self, params: mercury_capnp::home_session::PingParams<>,
            mut results: mercury_capnp::home_session::PingResults<>)
        -> Promise<(), ::capnp::Error>
    {
        let ping = pry!( pry!( params.get() ).get_txt() );
        results.get().set_pong(ping);
        Promise::ok( () )
    }
}
