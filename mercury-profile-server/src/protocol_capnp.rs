use std::rc::Rc;

use mercury_common::mercury_capnp::*;

use super::*;



pub struct HomeDispatcher
{
    home: Rc<Home>,
}

impl HomeDispatcher
{
    pub fn new(home: Rc<Home>) -> Self
        { Self{ home: home } }
}

impl mercury_capnp::profile_repo::Server for HomeDispatcher
{
    fn list(&mut self,
            params: mercury_capnp::profile_repo::ListParams,
            mut results: mercury_capnp::profile_repo::ListResults,)
        -> Promise<(), ::capnp::Error>
    {
        // TODO
        Promise::result( Ok( () ) )
    }

    fn load(&mut self,
            params: mercury_capnp::profile_repo::LoadParams,
            mut results: mercury_capnp::profile_repo::LoadResults,)
        -> Promise<(), ::capnp::Error>
    {
        //let builder : ::capnp::message::Builder<::capnp::message::HeapAllocator>;
        //let builder : mercury_capnp::profile::Builder::new_default();

        let profile_id_capnp = pry!( pry!( params.get() ).get_profile_id() );
        let load_fut = self.home.load( &profile_id_capnp.into() )
            .map( move |profile| results.get().init_profile().fill_from(&profile) )
            .map_err( |e| ::capnp::Error::failed( "Failed".to_owned() ) ); // TODO proper error handling

        Promise::from_future(load_fut)
    }

    fn resolve(&mut self,
               params: mercury_capnp::profile_repo::ResolveParams,
               mut results: mercury_capnp::profile_repo::ResolveResults,)
        -> Promise<(), ::capnp::Error>
    {
        Promise::result( Ok( () ) )
    }
}

impl mercury_capnp::home::Server for HomeDispatcher
{
    fn login(&mut self,
             params: mercury_capnp::home::LoginParams,
             mut results: mercury_capnp::home::LoginResults,)
             -> Promise<(), ::capnp::Error>
    {
        let res = params.get()
            .and_then( |params| params.get_profile_id() )
            .and_then( |profile_id|
            {
                println!("login called with '{:?}', sending session", profile_id);
                let session = mercury_capnp::home_session::ToClient::new( HomeSessionDispatcher::new() )
                    .from_server::<::capnp_rpc::Server>();
                results.get().set_session(session);
                Ok( () )
            } );
        Promise::result(res)
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
            mut results: mercury_capnp::home_session::PingResults<>) ->
            Promise<(), ::capnp::Error>
    {
        let res = params.get()
            .and_then( |params| params.get_txt() )
            .and_then( |ping|
            {
                println!("ping called with '{}', sending pong", ping);
                results.get().set_pong(ping);
                Ok( () )
            } );
        Promise::result(res)
    }
}
