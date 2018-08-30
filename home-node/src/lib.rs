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
extern crate failure;

pub mod config;
pub mod protocol_capnp;
pub mod server;

