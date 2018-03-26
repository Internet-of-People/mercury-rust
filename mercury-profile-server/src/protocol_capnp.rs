use std::rc::Rc;

use super::*;



trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( futures::future::result(result) ) }
}

impl<T,E> PromiseUtil<T,E> for Promise<T,E> {}



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
