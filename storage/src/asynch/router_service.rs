// TODO this client and also the related network protocol (thus also the server)
//      seems to rely heavily on ProfileIds being sized constant 32-bytes.
//      As we're using multicipher profile cryptography now, this needs elaborate fixing.
//      Until then it's commented as a whole.
/*
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::io;
use std::io::{Read, Seek, Write};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::rc::Rc;
use std::str::FromStr;

use arrayref::array_ref;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use futures::future::{loop_fn, Either, Loop};
use futures::prelude::*;
use futures::stream::{SplitSink, SplitStream};
use futures::sync::mpsc;
use futures::sync::{oneshot, oneshot::Sender};
use log::*;
use multiaddr::{Multiaddr, ToMultiaddr};
use tokio_core::net::{UdpCodec, UdpFramed, UdpSocket};
use tokio_core::reactor;
use tokio_timer::Delay;

use crate::asynch::StorageResult;
use mercury_home_protocol::{
    net::HomeConnector, Profile, ProfileFacet, ProfileId, ProfileRepo, Signer,
};

use super::StorageError;
use crate::asynch::KeyValueStore;

const PROFILE_SIZE: usize = 32;

const ADD_PERSONAS_REQUEST_ID: u8 = 1;
const DROP_PERSONAS_REQUEST_ID: u8 = 2;
const SET_HOME_REQUEST_ID: u8 = 3;
const HOME_ADDRESSES_QUERY_ID: u8 = 4;
const PROFILE_HOMES_QUERY_ID: u8 = 5;

fn from_slice(v: &[u8]) -> [u8; 32] {
    if v.len() != 32 {
        panic!(format!("unexpected profile id size: {}", v.len()));
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
        match &msg.serialize() {
            Ok(msgbuf) => {
                buf.clone_from(msgbuf);
                self.dest.clone()
            }
            Err(e) => {
                error!("Failed to serialize message: {}", e);
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            }
        }
    }
}

///
/// RequestReplySocket is responsible for serializing requests and waiting for the responses
///
struct RequestReplySocket {
    // 1. socket + codec
    // 2. pending requests in a map
    last_nonce: u64,
    sessions: HashMap<u64, Box<Sender<Reply>>>,
}

impl RequestReplySocket {
    pub fn new() -> Self {
        let last_nonce = 0;
        let sessions = HashMap::new();
        RequestReplySocket { last_nonce, sessions }
    }

    pub fn query<T: MercuryRouterQuery>(&mut self, qry: Query<T>) -> StorageResult<Reply> {
        // let request = Request::new();
        let (tx, rx) = oneshot::channel();
        Box::new(rx.map_err(|err| StorageError::StringError(err.description().to_string())))
    }
}

pub struct RouterServiceClient {
    host: Box<KeyValueStore<ProfileId, Profile>>,
    handle: reactor::Handle,
    server_address: SocketAddr,
    sock: Rc<RefCell<RequestReplySocket>>,
    home_connector: Rc<HomeConnector>,
    signer: Rc<Signer>,
}

impl RouterServiceClient {
    pub fn new(
        handle: reactor::Handle,
        host: Box<KeyValueStore<ProfileId, Profile>>,
        server_address: SocketAddr,
        home_connector: Rc<HomeConnector>,
        signer: Rc<Signer>,
    ) -> std::io::Result<Self> {
        let sock = Rc::new(RefCell::new(RequestReplySocket::new()));
        Ok(RouterServiceClient { host, handle, server_address, sock, home_connector, signer })
    }
}

impl KeyValueStore<ProfileId, Profile> for RouterServiceClient {
    fn set(&mut self, profile_id: ProfileId, profile: Profile) -> StorageResult<()> {
        unimplemented!();
    }

    fn get(&self, key: ProfileId) -> StorageResult<Profile> {
        // 1. issue a profile_homes request
        let p = from_slice(key.0.as_slice());
        let query = ProfileHomes::new(p);
        let nonce: u64 = 1;
        let query = Query::new(query, nonce);

        let connector = self.home_connector.clone();
        let signer = self.signer.clone();
        let get_fut = self
            .sock
            .borrow_mut()
            .query(query)
            .map_err(|err| StorageError::StringError("query failed".to_string()))
            .and_then(move |reply| {
                // process reply
                if reply.code != 0 {
                    return Either::A(
                        Err(StorageError::StringError(
                            "request failed with an error code".to_string(),
                        ))
                        .into_future(),
                    );
                }

                // 2. on success create a Home around the remote home
                let payload = match reply.payload {
                    Some(p) => p,
                    None => {
                        return Either::A(
                            Err(StorageError::StringError("no payload in reply".to_string()))
                                .into_future(),
                        );
                    }
                };

                match payload {
                    ReplyPayload::Addresses(addrs) => {
                        let profile_fut = connector
                            .connect_to_addrs(addrs.as_slice(), signer)
                            .map_err(|e| StorageError::StringError(e.to_string()))
                            .and_then(move |home| {
                                home.load(&key)
                                    .map_err(|e| StorageError::StringError(e.to_string()))
                            });
                        Either::B(profile_fut)
                    }
                    _ => Either::A(
                        Err(StorageError::StringError("invalid payload type".to_string()))
                            .into_future(),
                    ),
                }
            });
        Box::new(get_fut)
    }

    fn clear_local(&mut self, key: ProfileId) -> StorageResult<()> {
        unimplemented!();
    }
}

pub trait RequestPayload {
    fn request_type_id(&self) -> u8;
    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>>;
}

pub struct Request {
    signer: Rc<Signer>,
    nonce: u64,
    payload: Box<RequestPayload>,
}

impl Request {
    pub fn new(signer: Rc<Signer>, payload: Box<RequestPayload>, nonce: u64) -> Self {
        Self { signer, nonce, payload }
    }

    pub fn serialize(&self) -> io::Result<Vec<u8>> {
        let mut retval = io::Cursor::new(Vec::new());

        // request_id
        retval.write_u8(self.payload.request_type_id())?;
        eprintln!("request id pos: {}", retval.position());
        // nonce
        retval.write_u64::<BigEndian>(self.nonce)?;
        eprintln!("request id pos: {}", retval.position());

        let home_id = self.signer.profile_id();
        retval.write_all(&home_id.0)?;
        eprintln!("home id pos: {}", retval.position());

        // payload
        retval = self.payload.serialize(retval)?;
        eprintln!("payload pos: {}", retval.position());

        // pk
        retval.write_all(&self.signer.public_key().0)?;
        eprintln!("public key pos: {}", retval.position());

        // sig
        let signature = self.signer.sign(retval.get_ref());
        retval.write_all(&signature.0)?;
        eprintln!("sign pos: {}", retval.position());

        Ok(retval.into_inner())
    }
}

pub struct SetHomeRequest {
    addresses: Vec<Multiaddr>,
}

impl SetHomeRequest {
    pub fn new(addresses: Vec<Multiaddr>) -> Self {
        Self { addresses }
    }
}

impl RequestPayload for SetHomeRequest {
    fn request_type_id(&self) -> u8 {
        SET_HOME_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.addresses.iter().fold(
            Ok(out),
            |out: io::Result<io::Cursor<Vec<u8>>>, addr: &Multiaddr| {
                out.and_then(|mut out| {
                    let mut v = addr.to_string().as_bytes().to_vec();
                    v.truncate(255);
                    out.write_u8(v.len() as u8)?;
                    out.write_all(&v)?;
                    Ok(out)
                })
            },
        )
    }
}

pub struct AddPersonasRequest {
    profiles: Vec<[u8; PROFILE_SIZE]>,
}

impl AddPersonasRequest {
    pub fn new(profiles: Vec<[u8; PROFILE_SIZE]>) -> Self {
        Self { profiles }
    }
}

impl RequestPayload for AddPersonasRequest {
    fn request_type_id(&self) -> u8 {
        ADD_PERSONAS_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.profiles.iter().fold(Ok(out), |out: io::Result<io::Cursor<Vec<u8>>>, profile| {
            out.and_then(|mut out| {
                out.write_all(profile)?;
                Ok(out)
            })
        })
    }
}

pub struct DropPersonasRequest {
    profiles: Vec<[u8; PROFILE_SIZE]>,
}

impl DropPersonasRequest {
    pub fn new(profiles: Vec<[u8; PROFILE_SIZE]>) -> Self {
        Self { profiles }
    }
}

impl RequestPayload for DropPersonasRequest {
    fn request_type_id(&self) -> u8 {
        DROP_PERSONAS_REQUEST_ID
    }

    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        self.profiles.iter().fold(Ok(out), |out: io::Result<io::Cursor<Vec<u8>>>, profile| {
            out.and_then(|mut out| {
                out.write_all(profile)?;
                Ok(out)
            })
        })
    }
}

pub trait MercuryRouterQuery {
    fn request_type_id(&self) -> u8;
    fn serialize(&self, out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>>;
}

pub struct Query<T: Sized + MercuryRouterQuery> {
    nonce: u64,
    payload: T,
}

impl<T> Query<T>
where
    T: Sized + MercuryRouterQuery,
{
    pub fn new(payload: T, nonce: u64) -> Self {
        Self { nonce, payload }
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
    profile: [u8; PROFILE_SIZE],
}

impl ProfileHomes {
    pub fn new(profile: [u8; PROFILE_SIZE]) -> Self {
        Self { profile }
    }
}

impl MercuryRouterQuery for ProfileHomes {
    fn request_type_id(&self) -> u8 {
        PROFILE_HOMES_QUERY_ID
    }

    fn serialize(&self, mut out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        out.write_all(&self.profile)?;
        Ok(out)
    }
}

pub struct HomeAddresses {
    home_id: [u8; PROFILE_SIZE],
}

impl HomeAddresses {
    pub fn new(home_id: [u8; PROFILE_SIZE]) -> Self {
        Self { home_id }
    }
}

impl MercuryRouterQuery for HomeAddresses {
    fn request_type_id(&self) -> u8 {
        HOME_ADDRESSES_QUERY_ID
    }

    fn serialize(&self, mut out: io::Cursor<Vec<u8>>) -> io::Result<io::Cursor<Vec<u8>>> {
        out.write_all(&self.home_id)?;
        Ok(out)
    }
}

enum ReplyPayload {
    Profiles(Vec<[u8; 32]>),
    Addresses(Vec<Multiaddr>),
}

pub struct Reply {
    nonce: u64,
    code: u8,
    msg: String,
    payload: Option<ReplyPayload>,
}

impl Reply {
    pub fn parse<T>(data: &mut io::Cursor<T>) -> io::Result<Self>
    where
        T: AsRef<[u8]>,
    {
        let request_type_id = data.read_u8()?;
        let nonce = data.read_u64::<BigEndian>()?;
        let code = data.read_u8()?;
        let msgsize = data.read_u8()?;
        // let msg_raw : [u8; msgsize];
        // data.read_exact(msg_raw);
        let mut msg_raw: Vec<u8> = Vec::new();
        msg_raw.resize(msgsize as usize, 0);
        data.read_exact(&mut msg_raw)?;
        let msg = String::from_utf8(msg_raw)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

        let mut payload = None;
        match request_type_id {
            PROFILE_HOMES_QUERY_ID => {
                //
            }
            HOME_ADDRESSES_QUERY_ID => {
                let pos = data.position();
                data.seek(io::SeekFrom::End(0))?;
                let endpos = data.position();
                data.seek(io::SeekFrom::Start(pos));
                let mut addrs = Vec::new();

                while data.position() < endpos {
                    let addr_size = data.read_u8()?;
                    if (addr_size as u64 > endpos - data.position()) {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "unexpected end of packet",
                        ));
                    }
                    let mut addr_raw: Vec<u8> = Vec::new();
                    addr_raw.resize(addr_size as usize, 0);
                    data.read_exact(&mut addr_raw)?;
                    let addr = String::from_utf8(addr_raw)
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
                    let addr = Multiaddr::from_str(addr.as_str())
                        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
                    addrs.push(addr);
                }

                payload = Some(ReplyPayload::Addresses(addrs));
            }
            _ => {}
        }
        Ok(Self { nonce, code, msg, payload })
    }
}

impl fmt::Display for Reply {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Use `self.number` to refer to each positional data point.
        write!(f, "code: {} msg: '{}'", self.code, self.msg)
    }
}
*/
