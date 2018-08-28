extern crate bincode;
extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
extern crate clap;
extern crate futures;
#[macro_use]
extern crate log;
extern crate mercury_home_protocol;
extern crate mercury_storage;
extern crate multiaddr;
extern crate tokio_core;
extern crate tokio_io;
extern crate toml;
#[macro_use] 
extern crate failure;

pub mod config;
pub mod protocol_capnp;
pub mod server;


use failure::*;

#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>
}

pub enum ErrorKind {
    #[fail(display= "connection to home failed")]
    ConnectionToHomeFailed,

    #[fail(display= "peer id retreival failed")]
    PeerIdRetreivalFailed,

    #[fail(display= "profile load failed")]
    FailedToLoadProfile,

    #[fail(display= "profile claim failed")]
    FailedToClaimProfile,

    #[fail(display= "profile mismatch")]
    ProfileMismatch,

    #[fail(display="public key mismatch")]
    PublicKeyMismatch,

    #[fail(display="signer mismatch")]
    SignerMismatch,

    #[fail(display="home id mismatch")]
    HomeIdMismatch,

    #[fail(display="relation type mismatch")]
    RelationTypeMismatch,
    
    #[fail(display="invalid signature")]
    InvalidSignature,

    #[fail(display="storage failed")]
    StorageFailed,

    #[fail(display="persona expected")]
    PersonaExpected,

    #[fail(display="already registered")]
    AlreadyRegistered,

    #[fail(display="peer not hosted here")]
    PeerNotHostedHere,

    #[fail(display="invalid relation proof")]
    InvalidRelationProof,

    #[fail(display="timeout failed")]
    TimeoutFailed,

    #[fail(display="failed to read response")]
    FailedToReadResponse,

    #[fail(display="failed to send")]
    FailedToSend
}

impl Fail for Error {
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for HomeProtocolError {
    fn from(kind: ErrorKind) -> Error {
        Error { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<HomeProtocolErrorKind>) -> HomeProtocolError {
        HomeProtocolError { inner: inner }
    }
}


