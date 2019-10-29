use std::convert::TryFrom;

use async_trait::async_trait;
use capnp::capability::Promise;
use failure::Fallible;
use tokio::future::FutureExt;
use tokio::net::tcp::TcpStream;
use tokio::runtime::current_thread as reactor;

use super::*;
use crate::mercury_capnp::FillFrom;
use claims::model::Link;
use claims::repo::ProfileExplorer;

pub struct HomeClientCapnProto {
    peer_ctx: PeerContext,
    repo: mercury_capnp::profile_repo::Client,
    home: mercury_capnp::home::Client,
}

impl HomeClientCapnProto {
    pub fn new<R, W>(peer_ctx: PeerContext, reader: R, writer: W) -> Self
    where
        R: futures::io::AsyncRead + Unpin + 'static,
        W: futures::io::AsyncWrite + Unpin + 'static,
    {
        debug!("Initializing Cap'n'Proto Home client");

        let rpc_network = Box::new(capnp_rpc::twoparty::VatNetwork::new(
            reader,
            writer,
            capnp_rpc::rpc_twoparty_capnp::Side::Client,
            Default::default(),
        ));
        let mut rpc_system = capnp_rpc::RpcSystem::new(rpc_network, None);

        let home: mercury_capnp::home::Client =
            rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);
        let repo: mercury_capnp::profile_repo::Client =
            rpc_system.bootstrap(capnp_rpc::rpc_twoparty_capnp::Side::Server);

        let rpc_loop_fut = async {
            match rpc_system.await {
                Ok(()) => info!("Capnp RPC finished"),
                Err(e) => warn!("Capnp RPC failed: {}", e),
            }
        };
        reactor::spawn(rpc_loop_fut);

        Self { peer_ctx, home, repo }
    }

    pub fn new_tcp(peer_ctx: PeerContext, tcp_stream: TcpStream) -> Self {
        // TODO consider if this unwrap() is acceptable here
        tcp_stream.set_nodelay(true).unwrap();

        let x = futures_tokio_compat::Compat::new(tcp_stream);
        let (reader, writer) = x.split();
        HomeClientCapnProto::new(peer_ctx, reader, writer)
    }
}

#[async_trait(?Send)]
impl ProfileExplorer for HomeClientCapnProto {
    async fn fetch(&self, id: &ProfileId) -> Fallible<Profile> {
        let mut request = self.repo.get_request();
        request.get().set_profile_id(&id.to_bytes());

        let resp = request.send().promise.await.map_err_fail(ErrorKind::FailedToLoadProfile)?;

        let profile_capnp = resp.get()?.get_profile()?;
        bytes_to_profile(profile_capnp).map_err_fail(ErrorKind::FailedToLoadProfile)
    }

    async fn followers(&self, _id: &ProfileId) -> Fallible<Vec<Link>> {
        unimplemented!()
    }
}

#[async_trait(?Send)]
impl Home for HomeClientCapnProto {
    async fn claim(&self, profile_id: ProfileId) -> Fallible<RelationProof> {
        let mut request = self.home.claim_request();
        request.get().set_profile_id(&profile_id.to_bytes());

        let resp = request.send().promise.await.map_err_fail(ErrorKind::FailedToClaimProfile)?;

        RelationProof::try_from(resp.get()?.get_hosting_proof()?)
            .map_err_fail(ErrorKind::FailedToClaimProfile)
    }

    async fn register(
        &self,
        half_proof: &RelationHalfProof,
        //invite: Option<HomeInvitation>,
    ) -> Fallible<RelationProof> {
        let mut request = self.home.register_request();
        request.get().init_half_proof().fill_from(half_proof);
        //if let Some(inv) = invite {
        //    request.get().init_invite().fill_from(inv);
        //}

        let resp = request.send().promise.await.map_err_fail(ErrorKind::RegisterFailed)?;

        RelationProof::try_from(resp.get()?.get_hosting_proof()?)
            .map_err_fail(ErrorKind::RegisterFailed)
    }

    async fn login(&self, hosting_proof: &RelationProof) -> Fallible<Rc<dyn HomeSession>> {
        let mut request = self.home.login_request();
        request.get().init_hosting_proof().fill_from(hosting_proof);

        let resp = request.send().promise.await.map_err_fail(ErrorKind::FailedToCreateSession)?;

        let res = Rc::new(HomeSessionClientCapnProto::new(resp.get()?.get_session()?))
            as Rc<dyn HomeSession>;
        Ok(res)
    }

    // NOTE acceptor must have this server as its home
    async fn pair_request(&self, half_proof: &RelationHalfProof) -> Fallible<()> {
        let mut request = self.home.pair_request_request();
        request.get().init_half_proof().fill_from(half_proof);

        request.send().promise.await.map_err_fail(ErrorKind::PairRequestFailed)?;

        Ok(())
    }

    // NOTE acceptor must have this server as its home
    async fn pair_response(&self, relation_proof: &RelationProof) -> Fallible<()> {
        let mut request = self.home.pair_response_request();
        request.get().init_relation().fill_from(relation_proof);

        request.send().promise.await.map_err_fail(ErrorKind::PairResponseFailed)?;

        Ok(())
    }

    async fn call(
        &self,
        app: ApplicationId,
        call_req: &CallRequestDetails,
    ) -> Fallible<Option<AppMsgSink>> {
        let mut request = self.home.call_request();
        request.get().init_relation().fill_from(&call_req.relation);
        request.get().set_app((&app).into());
        request.get().set_init_payload((&call_req.init_payload).into());

        if let Some(send) = call_req.to_caller.as_ref() {
            let to_caller_dispatch =
                mercury_capnp::AppMessageDispatcherCapnProto::new(send.to_owned());
            let to_caller_capnp =
                mercury_capnp::app_message_listener::ToClient::new(to_caller_dispatch)
                    .into_client::<::capnp_rpc::Server>();
            request.get().set_to_caller(to_caller_capnp);
        }

        let resp = request.send().promise.await.map_err_fail(ErrorKind::CallFailed)?;

        let to_callee: Option<_> = resp.get()?.get_to_callee().ok();
        let res = to_callee.map(mercury_capnp::fwd_appmsg);
        Ok(res)
    }
}

struct ProfileEventDispatcherCapnProto {
    sender: mpsc::Sender<Result<ProfileEvent, String>>,
}

impl ProfileEventDispatcherCapnProto {
    fn new(sender: mpsc::Sender<Result<ProfileEvent, String>>) -> Self {
        Self { sender }
    }
}

impl mercury_capnp::profile_event_listener::Server for ProfileEventDispatcherCapnProto {
    fn receive(
        &mut self,
        params: mercury_capnp::profile_event_listener::ReceiveParams,
        _results: mercury_capnp::profile_event_listener::ReceiveResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.to_owned();
        let f = async move {
            let event_capnp = params.get()?.get_event()?;
            let event = ProfileEvent::try_from(event_capnp)?;
            trace!("Capnp client received event: {:?}", event);
            sender.send(Ok(event)).await.map_err_capnp("Failed to delegate event")
        };
        Promise::from_future(f)
    }

    fn error(
        &mut self,
        params: mercury_capnp::profile_event_listener::ErrorParams,
        _results: mercury_capnp::profile_event_listener::ErrorResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.to_owned();
        let f = async move {
            let error = params.get()?.get_error()?;
            trace!("Capnp client received error: {:?}", error);
            sender.send(Err(error.to_owned())).await.map_err_capnp("Failed to delegate error")
        };
        Promise::from_future(f)
    }
}

pub struct HomeSessionClientCapnProto {
    session: mercury_capnp::home_session::Client,
}

impl HomeSessionClientCapnProto {
    pub fn new(session: mercury_capnp::home_session::Client) -> Self {
        Self { session }
    }
}

#[async_trait(?Send)]
impl HomeSession for HomeSessionClientCapnProto {
    async fn backup(&self, own_prof: OwnProfile) -> Fallible<()> {
        let mut request = self.session.backup_request();
        request.get().set_own_profile(&own_profile_to_bytes(&own_prof));

        let _resp = request.send().promise.await.map_err_fail(ErrorKind::ProfileUpdateFailed)?;

        Ok(())
    }

    async fn restore(&self) -> Fallible<OwnProfile> {
        let request = self.session.restore_request();

        let resp = request.send().promise.await.map_err_fail(ErrorKind::ProfileLookupFailed)?;

        bytes_to_own_profile(resp.get()?.get_own_profile()?)
            .map_err_fail(ErrorKind::ProfileLookupFailed)
    }

    // NOTE new_home is a profile that contains at least one HomeFacet different than this home
    async fn unregister(&self, new_home: Option<Profile>) -> Fallible<()> {
        let mut request = self.session.unregister_request();
        if let Some(new_home_profile) = new_home {
            request.get().set_new_home(&profile_to_bytes(&new_home_profile));
        }

        let _resp = request.send().promise.await.map_err_fail(ErrorKind::UnregisterFailed)?;

        Ok(())
    }

    fn events(&self) -> AsyncStream<ProfileEvent, String> {
        let (mut send, recv) = mpsc::channel(1);
        let listener = ProfileEventDispatcherCapnProto::new(send.clone());
        // TODO consider how to drop/unregister this object from capnp if the stream is dropped
        let listener_capnp = mercury_capnp::profile_event_listener::ToClient::new(listener)
            .into_client::<::capnp_rpc::Server>();

        let mut request = self.session.events_request();
        request.get().set_event_listener(listener_capnp);

        // This future translates an event registration error into a "remote error" on the same
        // stream as is returned to the caller
        let registration_fut = async move {
            match request.send().promise.await {
                Ok(_r) => {}
                Err(e) => {
                    // TODO what to do if failed to send error?
                    let _res = send.send(Err(format!("Events delegation failed: {}", e))).await;
                }
            };
        };
        reactor::spawn(registration_fut);

        recv
    }

    fn checkin_app(&self, app: &ApplicationId) -> AsyncStream<Box<dyn IncomingCall>, String> {
        // Send a call dispatcher proxy to remote home through which we'll accept incoming calls
        let (mut send, recv) = mpsc::channel(1);
        let listener = CallDispatcherCapnProto::new(send.clone());
        // TODO consider how to drop/unregister this object from capnp if the stream is dropped
        let listener_capnp = mercury_capnp::call_listener::ToClient::new(listener)
            .into_client::<::capnp_rpc::Server>();

        let mut request = self.session.checkin_app_request();
        request.get().set_app(app.into());
        request.get().set_call_listener(listener_capnp);

        // We can either return Future<Stream> or
        // return the stream directly and spawn sending the request in another fiber
        let registration_fut = async move {
            match request.send().promise.await {
                Ok(_r) => {}
                Err(e) => {
                    // TODO what to do if failed to send error?
                    let _res = send.send(Err(format!("Call delegation failed: {}", e))).await;
                }
            };
        };
        reactor::spawn(registration_fut);

        recv
    }

    async fn ping(&self, txt: &str) -> Fallible<String> {
        let mut request = self.session.ping_request();
        request.get().set_txt(txt);

        let resp = request.send().promise.await.map_err_fail(ErrorKind::PingFailed)?;

        let res = resp.get()?.get_pong()?.to_owned();
        Ok(res)
    }
}

const CALL_TIMEOUT_SECS: u64 = 30;

struct CallDispatcherCapnProto {
    sender: mpsc::Sender<Result<Box<dyn IncomingCall>, String>>,
}

impl CallDispatcherCapnProto {
    fn new(sender: mpsc::Sender<Result<Box<dyn IncomingCall>, String>>) -> Self {
        Self { sender }
    }
}

impl mercury_capnp::call_listener::Server for CallDispatcherCapnProto {
    // Receive notification on an incoming call request and
    // send back a message channel if answering the call
    fn receive(
        &mut self,
        params: mercury_capnp::call_listener::ReceiveParams,
        mut results: mercury_capnp::call_listener::ReceiveResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.to_owned();

        let f = async move {
            // NOTE there's no way to add the i/o streams in try_from without extra context,
            //      we have to set them manually
            let call_capnp = params.get()?.get_call()?;
            let mut call = CallRequestDetails::try_from(call_capnp)?;
            // If received a to_caller channel, setup an in-memory sink for easier sending
            call.to_caller = call_capnp.get_to_caller().ok().map(mercury_capnp::fwd_appmsg);

            let (one_send, one_recv) = oneshot::channel::<Option<AppMsgSink>>();
            let answer_fut = async move {
                // TODO should we send an error back to the caller?
                let to_callee_opt =
                    one_recv.await.map_err_capnp("Failed to get answer from callee")?;

                to_callee_opt.map(|sink| results.get().set_to_callee(sink.into()));
                capnp::Result::Ok(())
            };

            // TODO make this timeout period user-configurable
            // Call will time out if not answered in a given period
            let answer_or_timeout_fut = answer_fut.timeout(Duration::from_secs(CALL_TIMEOUT_SECS));

            // Set up an IncomingCall object allowing to decide answering or refusing the call
            // TODO consider error handling: should we send error and close the sink in case of errors above?
            let incoming_call = Box::new(IncomingCallCapnProto::new(call, one_send));
            sender.send(Ok(incoming_call)).await.map_err_capnp("Failed to dispatch call")?;
            answer_or_timeout_fut.await.map_err_capnp("Failed to answer call")??; // timeout is Result<Result<_>>
            Ok(())
        };

        // TODO consider if the call (e.g. channels and capnp server objects) is dropped after a timeout
        //      but lives after properly accepted
        Promise::from_future(f)
    }

    fn error(
        &mut self,
        params: mercury_capnp::call_listener::ErrorParams,
        _results: mercury_capnp::call_listener::ErrorResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.to_owned();
        let f = async move {
            let error = params.get()?.get_error()?;

            sender
                .send(Err(error.to_owned()))
                .await
                .map_err_capnp("Failed to dispatch call error")?;

            Ok(())
        };
        Promise::from_future(f)
    }
}

struct IncomingCallCapnProto {
    request: CallRequestDetails,
    sender: oneshot::Sender<Option<AppMsgSink>>,
}

impl IncomingCallCapnProto {
    fn new(request: CallRequestDetails, sender: oneshot::Sender<Option<AppMsgSink>>) -> Self {
        Self { request, sender }
    }
}

impl IncomingCall for IncomingCallCapnProto {
    fn request_details(&self) -> &CallRequestDetails {
        &self.request
    }

    fn answer(self: Box<Self>, to_callee: Option<AppMsgSink>) -> CallRequestDetails {
        // NOTE needed to dereference Box because otherwise the whole self is moved at its first dereference
        let this = *self;
        match this.sender.send(to_callee) {
            Ok(()) => {}
            Err(_e) => {} // TODO what to do with the error? Only log or can we handle it somehow?
        };
        this.request
    }
}
