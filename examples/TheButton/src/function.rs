use std;

use futures;
use futures::{Future};
use tokio_signal;

use tokio_signal::unix::Signal;

pub type SigStream = futures::FlattenStream<
    std::boxed::Box<futures::Future<Error=std::io::Error, Item=tokio_signal::unix::Signal> 
    + std::marker::Send>
>;

pub fn signal_recv(sig : i32)-> SigStream{
    Signal::new(sig).flatten_stream()
}

