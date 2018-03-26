use super::*;
//use capnp::capability::Promise;
//use futures::Future;



trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( futures::future::result(result) ) }
}

impl<T,E> PromiseUtil<T,E> for Promise<T,E> {}



pub struct HomeImpl {}

impl HomeImpl
{
    pub fn new() -> Self { Self{} }
}

impl mercury_capnp::profile_repo::Server for HomeImpl {}

impl mercury_capnp::home::Server for HomeImpl
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
                let session = mercury_capnp::home_session::ToClient::new( HomeSessionImpl::new() )
                    .from_server::<::capnp_rpc::Server>();
                results.get().set_session(session);
                Ok( () )
            } );
        Promise::result(res)
    }
}



pub struct HomeSessionImpl {}

impl HomeSessionImpl
{
    pub fn new() -> Self { Self{} }
}

impl mercury_capnp::home_session::Server for HomeSessionImpl
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
