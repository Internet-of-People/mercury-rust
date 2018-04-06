#![allow(unused)]
extern crate capnp;
#[macro_use]
extern crate capnp_rpc;
extern crate futures;
extern crate mercury_common;
extern crate multiaddr;
extern crate multihash;
extern crate tokio_core;
extern crate tokio_io;



pub mod protocol_capnp;
pub mod server;



use std::rc::Rc;

use futures::{Future, Stream};
use tokio_core::reactor;
use tokio_core::net::TcpListener;
use tokio_io::AsyncRead;

use mercury_common::*;