pub mod api;
pub mod crypto;
pub mod error;
pub mod future;
pub mod handshake;
pub mod mercury_capnp;
pub mod net;
pub mod primitives;
pub mod util;

use std::rc::Rc;
use std::time::Duration;

use futures::prelude::*;
use futures::sync::{mpsc, oneshot};
use log::*;
use serde_derive::{Deserialize, Serialize};

pub use crate::api::*;
pub use crate::crypto::{Signer, Validator};
pub(crate) use crate::error::*;
pub use crate::primitives::*;
pub use did::model::{AsyncFallible, AsyncResult};
pub use did::repo::{
    DistributedPublicProfileRepository, PrivateProfileRepository, ProfileExplorer,
};
pub use keyvault;
pub use keyvault::ed25519;

pub const CHANNEL_CAPACITY: usize = 1;
