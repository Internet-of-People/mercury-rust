//#![warn(rust_2018_idioms)]

pub mod api;
pub mod crypto;
pub mod error;
pub mod handshake;
pub mod mercury_capnp;
pub mod primitives;
pub mod util;

use std::rc::Rc;
use std::time::Duration;

use failure::{Fail, Fallible};
use log::*;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

pub use crate::api::*;
pub use crate::crypto::Validator;
pub(crate) use crate::error::*;
pub use crate::primitives::*;
pub use claims::repo::{
    DistributedPublicProfileRepository, PrivateProfileRepository, ProfileExplorer,
};
pub use did::*;
pub use keyvault;
pub use keyvault::ed25519;

pub const CHANNEL_CAPACITY: usize = 1;

pub trait FailExt<T>
where
    Self: Sized,
{
    fn map_err_fail(self, k: ErrorKind) -> Fallible<T>;

    fn map_err_dh(self) -> Fallible<T> {
        self.map_err_fail(ErrorKind::DiffieHellmanHandshakeFailed)
    }
}

impl<T, E: Fail> FailExt<T> for Result<T, E> {
    fn map_err_fail(self, k: ErrorKind) -> Fallible<T> {
        self.map_err(|e| e.context(k).into())
    }
}
