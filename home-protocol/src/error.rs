use std::fmt::Display;

use failure::{Backtrace, Context, Fail};



#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display= "profile lookup failed")]
    ProfileLookupFailed,
    #[fail(display="profile update failed")]
    ProfileUpdateFailed,
    #[fail(display= "hash decode failed")]
    HashDecodeFailed,
    #[fail(display= "hash encode failed")]
    HashEncodeFailed,
    #[fail(display= "signer creation failed")]
    SignerCreationFailed,
    #[fail(display= "signature validation failed")]
    SignatureValidationFailed,
    #[fail(display= "handshake failed")]
    DiffieHellmanHandshakeFailed,
    #[fail(display= "relation signing failed")]
    RelationSigningFailed,
    #[fail(display= "relation validation failed")]
    RelationValidationFailed,
    #[fail(display= "profile validation failed")]
    ProfileValidationFailed,
    #[fail(display= "multiaddress serialization failed")]
    MultiaddrSerializationFailed,
    #[fail(display= "multiaddress deserialization failed")]
    MultiaddrDeserializationFailed,
    #[fail(display= "failed to fetch peer id")]
    PeerIdRetreivalFailed,
    #[fail(display= "profile claim failed")]
    FailedToClaimProfile,
    #[fail(display="persona expected")]
    PersonaExpected,
    #[fail(display="already registered")]
    AlreadyRegistered,
    #[fail(display="home id mismatch")]
    HomeIdMismatch,
    #[fail(display="relation type mismatch")]
    RelationTypeMismatch,
    #[fail(display="invalid signature")]
    InvalidSignature,
    #[fail(display="storage failed")]
    StorageFailed,
    #[fail(display= "profile mismatch")]
    ProfileMismatch,
    #[fail(display="public key mismatch")]
    PublicKeyMismatch,
    #[fail(display="signer mismatch")]
    SignerMismatch,
    #[fail(display="peer not hosted here")]
    PeerNotHostedHere,
    #[fail(display="invalid relation proof")]
    InvalidRelationProof,
    #[fail(display="timeout failed")]
    TimeoutFailed,
    #[fail(display="failed to read response")]
    FailedToReadResponse,
    #[fail(display="deregistered")]
    ProfileDeregistered,
    #[fail(display= "profile load failed")]
    FailedToLoadProfile,
    #[fail(display="call failed")]
    CallFailed,
    #[fail(display="failed to push event")]
    FailedToPushEvent,
    #[fail(display= "connection to home failed")]
    ConnectionToHomeFailed,
    #[fail(display="failed to send")]
    FailedToSend,
    #[fail(display="context validation failed")]
    ContextValidationFailed,
    #[fail(display="failed to get session")]
    FailedToGetSession,
    #[fail(display="failed to resolve URL")]
    FailedToResolveUrl,
    #[fail(display="pair request failed")]
    PairRequestFailed,
    #[fail(display="pair response failed")]
    PairResponseFailed,
    #[fail(display="profile registration failed")]
    RegisterFailed,
    #[fail(display="profile deregistration failed")]
    UnregisterFailed,
    #[fail(display="failed to create session")]
    FailedToCreateSession,
    #[fail(display="DHT lookup failed")]
    DhtLookupFailed,
    #[fail(display="ping failed")]
    PingFailed,
    #[fail(display="login failed")]
    LoginFailed,
}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        self.inner.get_context() == other.inner.get_context()
    }
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
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error { inner: Context::new(kind) }
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner: inner }
    }
}
