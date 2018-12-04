extern crate multiaddr;
extern crate byteorder;
extern crate signatory;
extern crate signatory_dalek;
extern crate ed25519_dalek;
extern crate hex;
extern crate rand;
extern crate sha2;
extern crate sha3;
extern crate tokio_io;
extern crate tokio_timer;
extern crate tokio_core;

use std::io;
use std::io::{Write, Read};
use std::fmt;
use std::str::FromStr;
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::hash::{Hash, Hasher};

use self::tokio_core::reactor;
use self::tokio_core::net::{UdpSocket, UdpCodec, UdpFramed};
use self::tokio_timer::Delay;
use futures::sync::mpsc;
use futures::stream::{SplitSink, SplitStream};
use futures::future::{loop_fn, Loop};

use self::multiaddr::Multiaddr;
use self::byteorder::{WriteBytesExt, ReadBytesExt, BigEndian};
use self::sha2::Sha512;
use self::sha3::{Digest, Keccak256};
use self::signatory::ed25519::{FromSeed, Seed, Signer };
use self::signatory_dalek::Ed25519Signer;
use futures::prelude::*;
use mercury_home_protocol::{ProfileId, Profile, ProfileFacet, ProfileRepo};
use mercury_home_protocol::Signer as MercurySigner;
use async::AsyncResult;

use super::StorageError;

use async::KeyValueStore;

const PROFILE_SIZE : usize = 32;

const ADD_PERSONAS_REQUEST_ID : u8 = 1;
const DROP_PERSONAS_REQUEST_ID : u8 = 2;
const SET_HOME_REQUEST_ID : u8 = 3;
const HOME_ADDRESSES_QUERY_ID : u8 = 4;
const PROFILE_HOMES_QUERY_ID : u8 = 5;

struct RouterCodec {
    dest: SocketAddr,
}

impl UdpCodec for RouterCodec {
    type In = Reply;
    type Out = Request;

    fn decode(&mut self, _src: &SocketAddr, buf: &[u8]) -> io::Result<Reply> {
        Reply::parse(&mut io::Cursor::new(buf))
    }

    fn encode(&mut self, msg: Request, buf: &mut Vec<u8>) -> SocketAddr {
        buf.clone_from(&msg.serialize().unwrap() );
        self.dest.clone()
    }
}

pub struct RouterServiceClient{
    host: Box<KeyValueStore<ProfileId, Profile>>,
    handle: reactor::Handle,
    server_address: SocketAddr,

}



impl RouterServiceClient {
    pub fn new(handle : reactor::Handle, host: Box<KeyValueStore<ProfileId, Profile>>, server_address : SocketAddr) -> std::io::Result<Self> {
        Ok(RouterServiceClient { host, handle, server_address})
    }

/*
    pub fn new(handle : reactor::Handle, host_profile_repo: Box<KeyValueStore<ProfileId, Profile>>, server_address : SocketAddr) -> std::io::Result<Self> {
        let (tx_events, rx_events) = mpsc::channel(10);
        let sock = UdpSocket::bind(&SocketAddr::from_str("0.0.0.0:0").unwrap(), &handle)?;
        let (sock_sink, sock_stream) = sock.framed(RouterCodec { dest: server_address }).split();

        let (tx_nw_events, rx_nw_events) = mpsc::channel(10);
        handle.spawn(Self::service_loop(handle.clone()));
        Ok(Self {tx_events})
    }

    fn backend_handler() -> impl Future<Item=(), Error=()> {
        Ok(()).into_future()
    }

    fn service_loop(handle : reactor::Handle) -> impl Future<Item=(), Error=()> {
        // join ???
        handle.spawn(Self::proxy_handler());
        handle.spawn(Self::host_handler());



        service_handler()

//        rx_events.for_each(move |event| {
//            match event {
//                ServiceEvent::ProfileSyncRequest(profile) => {
//                    let profile_id = profile.id.clone();
//                    Box::new(host_profile_repo.set(profile_id, profile)
//                        .and_then(|| {
//                            let req = match profile.facet {
//                                ProfileFacet::Persona(facet) => {
//                                    // look up if we're hosting this profile version
//                                    // add_persona
//                                    Vec::new()
//                                },
//                                ProfileFacet::Home(facet) => {
//                                    // check if this home is the signer
//                                    // set_home
//                                    Vec::new()
//                                }
//                            }
//
//                            sock.send_dgram(dgram, server_addr)
//                                .map_err(|| /* timer + */)
//                        })
//                        .map_err(|_err| ())) as Box<Future<Item=_, Error=_>>
//                },
//                _ => {
//                    Box::new(Ok(()).into_future())
//                }
//            }
//
//        })
    }
*/
/*
    pub fn new(handle : reactor::Handle, host_profile_repo : Box<KeyValueStore<ProfileId, Profile>>, signer : Box<MercurySigner>) -> std::io::Result<Self> {
        /*
        let hack_server = SocketAddr::from_str("127.0.0.1:4545").unwrap();
        let addr = SocketAddr::from_str("0.0.0.0:0").unwrap();
        let sock = UdpSocket::bind(&addr, &handle.clone())?;
        let (tx, rx) = mpsc::channel(10);
        let nonce = 0 as u64; // TODO: timestamp
        let pending_requests = HashSet::new();
        handle.spawn(Self::sync(nonce, hack_server, sock, rx, pending_requests));
        */
        Ok(Self {host_profile_repo, dirty_queue: Box::new(tx), sock, nonce, pending_requests, signer, handle})
    }

    fn sync(nonce: u64, server: SocketAddr, sock: UdpSocket, rx: mpsc::Receiver<ProfileId>, pending_requests : HashSet<ServiceRequestState>) -> Box<Future<Item=(), Error=()>> {
        let res = loop_fn((nonce, sock, rx, pending_requests), |(mut nonce, sock, rx, mut pending_requests)| {
            let dirty_fut = rx.into_future()
                .and_then(|(id_opt, rx)| {
                    if let Some(id) = id_opt {
                        nonce += 1;
                        pending_requests.insert(ServiceRequestState {
                            nonce,
                            timeout: None,
                            profile_id: id,
                        });
                        let msg = Vec::new(); // TODO serialize requests
                        let sent_fut = sock.send_dgram(msg, server)
                            .and_then(|(sock, _)| Ok(Loop::Continue((nonce, sock, rx, pending_requests))).into_future());
                    }
                });
        });
        Box::new(res)
    }
*/
}

impl KeyValueStore<ProfileId,Profile>  for RouterServiceClient {
    fn set(&mut self, profile_id: ProfileId, profile: Profile) -> AsyncResult<()> {
        self.host.set(profile_id, profile.clone());

        match profile.facet {
            ProfileFacet::Persona(facet) => {
                unimplemented!()
            },
            ProfileFacet::Home(facet) => {
                unimplemented!()
            }
            _ => {
                unimplemented!()
            }
        }
    }

    fn get(&self, key: ProfileId) -> AsyncResult<Profile> {
        unimplemented!()
    }

    fn clear_local(&mut self, key: ProfileId) -> AsyncResult<()> {
        unimplemented!()
    }
}



pub trait RequestPayload {
    fn request_type_id(&self) -> u8;
    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>>;
}

pub struct Request {
    secret_key : ed25519_dalek::SecretKey,
    nonce : u64,
    payload : Box<RequestPayload>,
}

impl Request {
    pub fn new(secret_key : ed25519_dalek::SecretKey, payload: Box<RequestPayload>, nonce: u64) -> Self {
        Self{secret_key, nonce, payload}
    }


    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut retval = io::Cursor::new(Vec::new());

        // request_id
        retval.write_u8(self.payload.request_type_id())?;
        eprintln!("request id pos: {}", retval.position());
        // nonce
        retval.write_u64::<BigEndian>(self.nonce)?;
        eprintln!("request id pos: {}", retval.position());

        // home id
        let public_key = ed25519_dalek::PublicKey::from_secret::<Sha512>(&self.secret_key);
        let mut hasher = Keccak256::new();
        hasher.input(public_key.to_bytes());
        let home_id = hasher.result();

        retval.write_all(&home_id)?;
        eprintln!("home id pos: {}", retval.position());

        // payload
        retval = self.payload.serialize(retval)?;
        eprintln!("payload pos: {}", retval.position());

        let seed = Seed::from_slice(self.secret_key.as_bytes()).unwrap();
        let signer = Ed25519Signer::from_seed(seed);
        let signature = signer.sign(retval.get_ref()).unwrap();

        // pk
        retval.write_all(public_key.as_bytes())?;
        eprintln!("public key pos: {}", retval.position());

        // sig
        retval.write_all(signature.as_bytes())?;
        eprintln!("sign pos: {}", retval.position());

        Ok(retval.into_inner())
    }
}

pub struct SetHomeRequest {
    addresses : Vec<Multiaddr>
}

impl SetHomeRequest {
    pub fn new(addresses : Vec<Multiaddr>) -> Self {
        Self {addresses}
    }
}

impl RequestPayload for SetHomeRequest {
    fn request_type_id(&self) -> u8 {
        SET_HOME_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.addresses.iter().fold(Ok(out), |out : io::Result<io::Cursor<Vec<u8>>>, addr: &Multiaddr| {
            out.and_then(|mut out| {
                let mut v= addr.to_string().as_bytes().to_vec();
                v.truncate(255);
                out.write_u8(v.len() as u8)?;
                out.write_all(&v)?;
                Ok(out)
            })
        })
    }

}

pub struct AddPersonasRequest {
    profiles : Vec<[u8; PROFILE_SIZE]>,
}

impl AddPersonasRequest {
    pub fn new(profiles : Vec<[u8; PROFILE_SIZE]>) -> Self {
        Self{profiles}
    }
}

impl RequestPayload for AddPersonasRequest {
    fn request_type_id(&self) -> u8 {
        ADD_PERSONAS_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.profiles.iter().fold(Ok(out), |out : io::Result<io::Cursor<Vec<u8>>>, profile| {
            out.and_then(|mut out| {
                out.write_all(profile)?;
                Ok(out)
            })
        })
    }
}

pub struct DropPersonasRequest {
    profiles : Vec<[u8; PROFILE_SIZE]>
}

impl DropPersonasRequest {
    pub fn new(profiles : Vec<[u8; PROFILE_SIZE]>) -> Self {
        Self{profiles}
    }
}


impl RequestPayload for DropPersonasRequest {
    fn request_type_id(&self) -> u8 {
        DROP_PERSONAS_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.profiles.iter().fold(Ok(out), |out : io::Result<io::Cursor<Vec<u8>>>, profile| {
            out.and_then(|mut out| {
                out.write_all(profile)?;
                Ok(out)
            })
        })
    }
}



pub trait MercuryRouterQuery {
    fn request_type_id() -> u8;
    fn serialize(&self, out : io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>>;
}

pub struct Query <T : Sized + MercuryRouterQuery> {
    nonce : u64,
    payload : T

}

impl<T> Query<T> where T : Sized + MercuryRouterQuery {
    pub fn new(payload: T, nonce: u64) -> Self {
        Self{nonce, payload}
    }

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut retval = io::Cursor::new(Vec::new());
        // request type id
        retval.write_u8(T::request_type_id())?;

        // nonce
        retval.write_u64::<BigEndian>(self.nonce)?;

        // payload
        retval = self.payload.serialize(retval)?;

        Ok(retval.into_inner())
    }
}

pub struct ProfileHomes {
    profile : [u8; PROFILE_SIZE]
}

impl ProfileHomes {
    pub fn new(profile : [u8; PROFILE_SIZE]) -> Self{
        Self{profile}
    }
}

impl MercuryRouterQuery for ProfileHomes {
    fn request_type_id() -> u8 {
        PROFILE_HOMES_QUERY_ID
    }

    fn serialize(&self, mut out : io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        out.write_all(&self.profile)?;
        Ok(out)
    }
}

pub struct HomeAddresses {
    home_id : [u8; PROFILE_SIZE]
}

impl HomeAddresses {
    pub fn new(home_id : [u8; PROFILE_SIZE]) -> Self {
        Self{home_id}
    }

}

impl MercuryRouterQuery for HomeAddresses {
    fn request_type_id() -> u8 {
        HOME_ADDRESSES_QUERY_ID
    }

    fn serialize(&self, mut out : io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        out.write_all(&self.home_id)?;
        Ok(out)
    }
}

enum ReplyPayload {
    Profiles(Vec<[u8; 32]>),
    Addresses(Vec<String>)
}

pub struct Reply {
    nonce: u64,
    code: u8,
    msg: String,
    payload: Option<ReplyPayload>
}

impl Reply {
    pub fn parse<T>(data : &mut io::Cursor<T>)
        -> io::Result<Self>
    where T: AsRef<[u8]>
    {
        let request_type_id = data.read_u8()?;
        let nonce = data.read_u64::<BigEndian>()?;
        let code = data.read_u8()?;
        let msgsize = data.read_u8()?;
        // let msg_raw : [u8; msgsize];
        // data.read_exact(msg_raw);
        let mut msg_raw : Vec<u8> = Vec::new();
        msg_raw.resize(msgsize as usize, 0);
        data.read_exact(&mut msg_raw)?;
        let msg= String::from_utf8(msg_raw).map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let payload = None;
        match request_type_id {
            _ => {

            }
        }
        Ok(Self{nonce, code, msg, payload})
    }
}

impl fmt::Display for Reply {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "code: {} msg: '{}'", self.code, self.msg)
    }
}
