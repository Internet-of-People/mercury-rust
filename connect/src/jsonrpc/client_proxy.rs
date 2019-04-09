use std::cmp::min;
use std::io;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

//use failure::Fail;
use futures::prelude::*;
use futures::try_ready;
//use jsonrpc_core::{Metadata, MetaIoHandler, Params, serde_json as json, types};
//use jsonrpc_pubsub::{PubSubHandler, Session as PubSubSession, PubSubMetadata, Subscriber, SubscriptionId};
use log::*;
use state_machine_future::{transition, RentToOwn, StateMachineFuture};
//use tokio_codec::{Decoder, Encoder, Framed};
use tokio_core::{reactor, reactor::Timeout};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_uds::UnixStream;

use crate::*;
use mercury_home_protocol::{future::StreamWithDeadline, *};

const NETWORK_TIMEOUT: Duration = Duration::from_secs(5);

struct Context {
    handle: reactor::Handle,
    address: PathBuf,
    retries: u32,
}

impl Context {
    pub fn new(address: &Path, handle: &reactor::Handle) -> Self {
        Self { address: PathBuf::from(address), handle: handle.clone(), retries: 0 }
    }

    pub fn reset_backoff(&mut self) {
        self.retries = 0;
    }

    pub fn next_backoff_interval(&mut self) -> Duration {
        self.retries = min(self.retries + 1, 5);
        Duration::from_secs(2_u32.pow(self.retries - 1) as u64)
    }
}

struct DuplexChannel {
    rx: Box<AsyncRead>,
    tx: Box<AsyncWrite>,
}

impl DuplexChannel {
    fn new(rx: Box<AsyncRead>, tx: Box<AsyncWrite>) -> Self {
        Self { rx, tx }
    }
}

type ClientFsmError = io::Error;

#[derive(StateMachineFuture)]
#[state_machine_future(context = "Context")]
enum Peer {
    #[state_machine_future(start, transitions(Connecting))]
    Start(),

    #[state_machine_future(transitions(Connected, Backoff))]
    Connecting { fut: AsyncResult<DuplexChannel, ClientFsmError> },

    #[state_machine_future(transitions(Connecting))]
    Backoff { timer: AsyncResult<(), ClientFsmError> },

    #[state_machine_future(transitions(FinishedSuccess, Backoff))]
    Connected { channel: DuplexChannel },

    #[state_machine_future(ready)]
    FinishedSuccess(()),

    #[state_machine_future(error)]
    FinishedError(ClientFsmError),
}

impl Connecting {
    fn new(address: &Path, handle: &reactor::Handle) -> Self {
        let connect_fut = UnixStream::connect(address, handle).into_future().map(|stream| {
            let (rx, tx) = stream.split();
            DuplexChannel::new(Box::new(rx), Box::new(tx))
        });

        let timeout_fut = Timeout::new(NETWORK_TIMEOUT, handle)
            .into_future()
            .and_then(|_| Err(io::Error::from(io::ErrorKind::TimedOut)))
            .or_else(|err| Err(io::Error::new(io::ErrorKind::Other, err)));

        let fut = connect_fut
            .select(timeout_fut)
            .map(|(res, _pending)| res)
            .map_err(|(err, _pending)| err);

        Self { fut: Box::new(fut) }
    }
}

impl Backoff {
    fn new(duration: Duration, handle: &reactor::Handle) -> Self {
        Self { timer: Box::new(Timeout::new(duration, handle).unwrap()) }
    }
}

impl Connected {
    fn new(channel: DuplexChannel) -> Self {
        Self { channel }
    }
}

impl PollPeer for Peer {
    fn poll_start<'s, 'c>(
        _start: &'s mut RentToOwn<'s, Start>,
        context: &'c mut RentToOwn<'c, Context>,
    ) -> Poll<AfterStart, ClientFsmError> {
        transition!(Connecting::new(&context.address, &context.handle))
    }

    fn poll_connecting<'s, 'c>(
        connecting: &'s mut RentToOwn<'s, Connecting>,
        context: &'c mut RentToOwn<'c, Context>,
    ) -> Poll<AfterConnecting, ClientFsmError> {
        match connecting.fut.poll() {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(channel)) => {
                context.reset_backoff();
                transition!(Connected::new(channel))
            }
            Err(e) => {
                // TODO differentiate between timeouts and other failures, consider backoff only for timeouts
                let interval = context.next_backoff_interval();
                transition!(Backoff::new(interval, &context.handle))
            }
        }
    }

    fn poll_backoff<'s, 'c>(
        backoff: &'s mut RentToOwn<'s, Backoff>,
        context: &'c mut RentToOwn<'c, Context>,
    ) -> Poll<AfterBackoff, ClientFsmError> {
        try_ready!(backoff.timer.poll());
        transition!(Connecting::new(&context.address, &context.handle));
    }

    fn poll_connected<'s, 'c>(
        connected: &'s mut RentToOwn<'s, Connected>,
        context: &'c mut RentToOwn<'c, Context>,
    ) -> Poll<AfterConnected, ClientFsmError> {
        unimplemented!()
    }
}

pub struct DAppEndpointClient {}

impl DAppEndpointClient {
    fn new() -> Self {
        Self {}
    }
}

impl DAppEndpoint for DAppEndpointClient {
    fn dapp_session(
        &self,
        app: &ApplicationId,
        authorization: Option<DAppPermission>,
    ) -> AsyncResult<Rc<DAppSession>, Error> {
        unimplemented!()
    }
}

pub struct DAppSessionClient {}

impl DAppSession for DAppSessionClient {
    fn selected_profile(&self) -> ProfileId {
        unimplemented!()
    }

    // TODO merge these two operations using an optional profile argument
    fn contacts(&self) -> AsyncResult<Vec<Box<Contact>>, Error> {
        unimplemented!()
    }

    fn contacts_with_profile(
        &self,
        profile: &ProfileId,
        relation_type: Option<&str>,
    ) -> AsyncResult<Vec<Box<Contact>>, Error> {
        unimplemented!()
    }

    fn initiate_contact(&self, with_profile: &ProfileId) -> AsyncResult<(), Error> {
        unimplemented!()
    }

    fn app_storage(&self) -> AsyncResult<KeyValueStore<String, String>, Error> {
        unimplemented!()
    }

    fn checkin(&self) -> AsyncResult<Box<Stream<Item = DAppEvent, Error = ()>>, Error> {
        unimplemented!()
    }
}
