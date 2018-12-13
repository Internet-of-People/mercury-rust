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

use self::multiaddr::{Multiaddr, ToMultiaddr};
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

fn from_slice(v: &[u8]) -> [u8; 32] {
    if v.len() != 32 {
        panic!(format!("unexpected profile id size: {}",v.len()));
    }
    array_ref!(v, 0, 32).clone()
}

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


///
/// RequestReplySocket is responsible for serializing requests and waiting for the responses
///
struct RequestReplySocket {
    // 1. socket + codec
    // 2. pending requests in a map
}

impl RequestReplySocket {
    pub fn new() -> Self {
        unimplemented!();
    }

    pub fn query<T: MercuryRouterQuery>(&mut self, qry: &Query<T>) -> AsyncResult<Reply> {
        unimplemented!();
    }
}


pub struct RouterServiceClient{
    host: Box<KeyValueStore<ProfileId, Profile>>,
    handle: reactor::Handle,
    server_address: SocketAddr,
    sock : Rc<RefCell<RequestReplySocket>>,
}

impl RouterServiceClient {
    pub fn new(handle : reactor::Handle, host: Box<KeyValueStore<ProfileId, Profile>>, server_address : SocketAddr) -> std::io::Result<Self> {
        let sock = Rc::new(RefCell::new(RequestReplySocket::new()));
        Ok(RouterServiceClient { host, handle, server_address, sock})
    }

}

impl KeyValueStore<ProfileId,Profile>  for RouterServiceClient {
    fn set(&mut self, profile_id: ProfileId, profile: Profile) -> AsyncResult<()> {
        unimplemented!();
    }

    fn get(&self, key: ProfileId) -> AsyncResult<Profile> {
        // todo: 
        // 1. issue a profile_homes request
        let p = from_slice(key.0.as_slice());
        let query = ProfileHomes::new(p);
        let nonce : u64 = 1;
        let query = Query::new(query, nonce);


        Box::new(self.sock.borrow_mut()
            .query(&query)
            .map_err(|err| StorageError::StringError("query failed".to_string()))
            .and_then(|reply| {
                // process reply

                if reply.code != 0 {
                    return Err(StorageError::StringError("request failed with an error code".to_string()))
                }


                if reply.payload.is_none() {
                    return Err(StorageError::StringError("no payload in reply".to_string()))
                }

                // 2. on success create a Home around the remote home
                match reply.payload.unwrap() {
                    ReplyPayload::Addresses(addrs) => {
                        let addrs : Vec<Multiaddr>= addrs
                            .iter()
                            .map(|addrstr| {
                                addrstr.to_multiaddr().unwrap()
                            }).collect();
                        unimplemented!();
                    },
                    _ => {
                        return Err(StorageError::StringError("invalid payload type".to_string()));
                    }
                }

                unimplemented!();


            })
        )
    }

    fn clear_local(&mut self, key: ProfileId) -> AsyncResult<()> {
        unimplemented!();
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
    fn request_type_id(&self) -> u8;
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
        retval.write_u8(self.payload.request_type_id())?;

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
    fn request_type_id(&self) -> u8 {
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
    fn request_type_id(&self) -> u8 {
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
