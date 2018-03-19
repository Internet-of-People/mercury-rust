use std::net::{SocketAddr};

use mercury_common::mercury_capnp;
use futures::{Future};
use futures::future;
use multiaddr::{Multiaddr, Protocol};
use tokio_core::reactor;
use tokio_core::net::{TcpStream, TcpStreamNew};

use super::*;



pub struct HomeClientCapnProto
{
    // rpc_system: capnp_rpc::RpcSystem<capnp_rpc::rpc_twoparty_capnp::Side>,
    home:       mercury_capnp::home::Client<>,
}


impl HomeClientCapnProto
{
    pub fn new(tcp_stream: TcpStream, handle: reactor::Handle) -> Self
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

        handle.spawn( rpc_system.map_err( |e| println!("Capnp RPC failed: {}", e) ) );

        Self{ home: home } // , rpc_system: rpc_system
    }
}


impl ProfileRepo for HomeClientCapnProto
{
    fn list(&self, /* TODO what filter criteria should we have here? */ ) ->
        Box< Stream<Item=Profile, Error=ErrorToBeSpecified> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        Box::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }

    fn load(&self, id: &ProfileId) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    // NOTE should be more efficient than load(id) because URL is supposed to contain hints for resolution
    fn resolve(&self, url: &str) ->
        Box< Future<Item=Profile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }
}


impl Home for HomeClientCapnProto
{
    fn register(&self, own_prof: OwnProfile, invite: Option<HomeInvitation>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    // TODO consider if we should notify an open session about an updated profile
    fn update(&self, own_prof: OwnProfile) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    fn unregister(&self, own_prof: OwnProfile, newhome: Option<Profile>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    fn claim(&self, profile: Profile, signer: Rc<Signer>) ->
        Box< Future<Item=OwnProfile, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }


    // NOTE acceptor must have this server as its home
    fn pair_with(&self, initiator: OwnProfile, acceptor: Profile) ->
        Box< Future<Item=Contact, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }

    fn call(&self, caller: OwnProfile, callee: Contact,
            app: ApplicationId, init_payload: &[u8]) ->
        Box< Future<Item=CallMessages, Error=ErrorToBeSpecified> >
    {
        Box::new( futures::future::err(ErrorToBeSpecified::TODO) )
    }


    fn login(&self, own_prof: OwnProfile) ->
        Box< Future<Item=Box<HomeSession>, Error=ErrorToBeSpecified> >
    {
        println!("login() called");
        let mut request = self.home.login_request();
        request.get().set_name(&"beeeela"); // TODO
        println!("login request created");

        let resp_fut = request.send().promise
            .and_then( |resp|
            {
                println!("login() message sent");
                resp.get()
                    .and_then( |res| res.get_session() )
                    .map( |session_client| Box::new( HomeSessionClientCapnProto::new(session_client) ) )
                    .map( |session| session as Box<HomeSession>)
            } )
            .map_err( |e| { println!("login() failed {}", e); ErrorToBeSpecified::TODO } );;

        Box::new(resp_fut)
    }
}



pub struct HomeSessionClientCapnProto
{
    session: mercury_capnp::home_session::Client<>,
}

impl HomeSessionClientCapnProto
{
    pub fn new(session: mercury_capnp::home_session::Client) -> Self
        { Self{ session: session } }
}

impl HomeSession for HomeSessionClientCapnProto
{
    fn events(&self) -> Rc< Stream<Item=HomeEvent, Error=ErrorToBeSpecified> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        Rc::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }

    // TODO return not a Stream, but an AppSession struct containing a stream
    fn checkin_app(&self, app: &ApplicationId) ->
        Box< Stream<Item=Call, Error=ErrorToBeSpecified> >
    {
        let (send, recv) = futures::sync::mpsc::channel(0);
        Box::new( recv.map_err( |_| ErrorToBeSpecified::TODO ) )
    }

    fn ping(&self, txt: &str) ->
        Box< Future<Item=String, Error=ErrorToBeSpecified> >
    {
        println!("checkin() called");
        let mut request = self.session.ping_request();
        request.get().set_txt(txt);
        println!("checkin request created");

        let resp_fut = request.send().promise
            .and_then( |resp|
                {
                    println!("checkin() message sent");
                    resp.get()
                        .and_then( |res|
                            res.get_pong().map( |s| s.to_owned() ) )
                } )
            .map_err( |e| { println!("checkin() failed {}", e); ErrorToBeSpecified::TODO } );

        Box::new(resp_fut)
    }
}



// TODO this should return simply Rc<Home> but then it's a lot of boilerplate to compile until implemented
pub fn tcp_home(tcp_stream: TcpStream, handle: reactor::Handle) -> Rc<Home>
{
    Rc::new( HomeClientCapnProto::new(tcp_stream, handle) )
}



pub struct StunTurnTcpConnector
{
    // TODO
}


impl StunTurnTcpConnector
{
    pub fn connect(&self, addr: &SocketAddr) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        // TODO
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}



pub struct TcpHomeConnector
{
    // TODO
}


impl HomeConnector for TcpHomeConnector
{
    fn connect(&self, home_profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        // TODO in case of TCP addresses, use StunTurnTcpConnector to build an async TcpStream
        //      to it and build a Home proxy on top of it
        Box::new( future::err(ErrorToBeSpecified::TODO) )
    }
}



pub struct SimpleTcpHomeConnector
{
    handle: reactor::Handle,
}


impl SimpleTcpHomeConnector
{
    pub fn new(handle: reactor::Handle) -> Self
        { Self{ handle: handle} }

    pub fn connect_addr(addr: &Multiaddr, handle: &reactor::Handle) ->
        Box< Future<Item=TcpStream, Error=ErrorToBeSpecified> >
    {
        // TODO handle other multiaddresses, not only TCP
        let tcp_addr = match multiaddr_to_socketaddr(addr)
        {
            Ok(res) => res,
            Err(_) => return Box::new( future::err(ErrorToBeSpecified::TODO) )
        };

        let tcp_str = TcpStream::connect(&tcp_addr, handle)
            .map_err( |_| ErrorToBeSpecified::TODO );
        Box::new(tcp_str)
    }

}


impl HomeConnector for SimpleTcpHomeConnector
{
    fn connect(&self, home_profile: &Profile) ->
        Box< Future<Item=Rc<Home>, Error=ErrorToBeSpecified> >
    {
        let handle_clone = self.handle.clone();
        let tcp_conns = home_profile.facets.iter()
            .flat_map( |facet|
                match facet {
                    &ProfileFacet::Home(ref home) => home.addrs.clone(),
                    _ => Vec::new()
                }
            )
            .map(  move |addr| SimpleTcpHomeConnector::connect_addr(&addr, &handle_clone) );

        let handle_clone = self.handle.clone();
        let tcp_home = future::select_ok(tcp_conns)
            .map( move |(tcp, _pending_futs)| tcp_home(tcp, handle_clone) );
        Box::new(tcp_home)
    }
}
