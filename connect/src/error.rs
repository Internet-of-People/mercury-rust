use std::fmt::Display;

use failure::{Backtrace, Context, Fail};



#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    #[fail(display= "connection to home failed")]
    ConnectionToHomeFailed,

    #[fail(display="handshake failed")]
    HandshakeFailed,

    #[fail(display= "peer id retreival failed")]
    PeerIdRetreivalFailed,

    #[fail(display= "failed to get contacts")]
    FailedToGetContacts,

    #[fail(display="failed to get session")]
    FailedToGetSession,

    #[fail(display="address conversion failed")]
    AddressConversionFailed,

    #[fail(display="failed to connect tcp stream")]
    ConnectionFailed,

    #[fail(display="failed to load profile")]
    FailedToLoadProfile,

    #[fail(display="failed to resolve profile")]
    FailedToResolveProfile,

    #[fail(display="home profile expected")]
    HomeProfileExpected,

    #[fail(display="failed to claim profile")]
    FailedToClaimProfile,

    #[fail(display="registration failed")]
    RegistrationFailed,

    #[fail(display="deregistration failed")]
    DeregistrationFailed,

    #[fail(display="pair request failed")]
    PairRequestFailed,

    #[fail(display="peer response failed")]
    PeerResponseFailed,

    #[fail(display="profile update failed")]
    ProfileUpdateFailed,

    #[fail(display="call failed")]
    CallFailed,

    #[fail(display="call refused")]
    CallRefused,

    #[fail(display="lookup failed")]
    LookupFailed,

    #[fail(display="no proof found for home")]
    HomeProofNotFound,

    #[fail(display="persona profile expected")]
    PersonaProfileExpected,

    #[fail(display="no homes found")]
    NoHomesFound,

    #[fail(display="login failed")]
    LoginFailed,

    #[fail(display="failed to get peer id")]
    FailedToGetPeerId,

    #[fail(display="failed to authorize")]
    FailedToAuthorize,

    #[fail(display="implementation error")]
    ImplementationError,
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
