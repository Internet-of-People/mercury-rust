extern crate capnp;
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_common;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;

use std::rc::Rc;

use capnp::capability::Promise;
use futures::{Future, Stream};
use tokio_core::reactor;
use tokio_core::net::TcpListener;
use tokio_io::AsyncRead;

use mercury_common::*;
//use mercury_common::mercury_capnp;



trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( futures::future::result(result) ) }
}

impl<T,E> PromiseUtil<T,E> for Promise<T,E> {}



struct HomeImpl {}

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
            .and_then( |params| params.get_name() )
            .and_then( |name|
            {
                println!("login called with '{}', sending session", name);
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



fn main()
{
    let mut core = reactor::Core::new().unwrap();
    let handle = core.handle();

    use std::net::ToSocketAddrs;
    let addr = "localhost:9876".to_socket_addrs().unwrap().next().expect("Failed to parse address");
    let socket = TcpListener::bind(&addr, &handle).expect("Failed to bind socket");

    let home_impl = HomeImpl::new();
    let home = mercury_capnp::home::ToClient::new(home_impl)
        .from_server::<::capnp_rpc::Server>();

    println!("Waiting for clients");
    let handle1 = handle.clone();
    let done = socket.incoming().for_each(move |(socket, _addr)|
    {
        println!("Accepted client connection, serving requests");
        try!(socket.set_nodelay(true));
        let (reader, writer) = socket.split();
        let handle = handle1.clone();

        let network = capnp_rpc::twoparty::VatNetwork::new( reader, writer,
            capnp_rpc::rpc_twoparty_capnp::Side::Server, Default::default() );

        let rpc_system = capnp_rpc::RpcSystem::new( Box::new(network), Some(home.clone().client) );

        handle.spawn(rpc_system.map_err(|_| ()));
        Ok(())
    } );

    core.run(done).unwrap();
}
