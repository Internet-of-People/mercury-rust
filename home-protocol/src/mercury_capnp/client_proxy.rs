use std::convert::TryFrom;

use capnp::capability::Promise;
use capnp_rpc::pry;
use failure::Fail;
use tokio_core::net::TcpStream;
use tokio_core::reactor;

use super::*;
use crate::mercury_capnp::{FillFrom, PromiseUtil};
use did::model::Link;
use did::repo::ProfileExplorer;

pub struct HomeClientCapnProto {
    repo: mercury_capnp::profile_repo::Client,
    home: mercury_capnp::home::Client,
    handle: reactor::Handle,
}

impl HomeClientCapnProto {
    pub fn new<R, W>(reader: R, writer: W, handle: reactor::Handle) -> Self
    where
        R: std::io::Read + 'static,
        W: std::io::Write + 'static,
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

        handle.spawn(rpc_system.map_err(|e| warn!("Capnp RPC failed: {}", e)));

        Self { home, repo, handle }
    }

    pub fn new_tcp(tcp_stream: TcpStream, handle: reactor::Handle) -> Self {
        use tokio_io::AsyncRead;

        // TODO consider if this unwrap() is acceptable here
        tcp_stream.set_nodelay(true).unwrap();
        let (reader, writer) = tcp_stream.split();
        HomeClientCapnProto::new(reader, writer, handle)
    }
}

impl ProfileExplorer for HomeClientCapnProto {
    fn fetch(&self, id: &ProfileId) -> AsyncFallible<Profile> {
        let mut request = self.repo.get_request();
        request.get().set_profile_id(&id.to_bytes());

        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| {
                let profile_capnp = pry!(pry!(resp.get()).get_profile());
                let profile = bytes_to_profile(profile_capnp);
                Promise::result(profile)
            })
            .map_err(|e| e.context(ErrorKind::FailedToLoadProfile).into());

        Box::new(resp_fut)
    }

    fn followers(&self, _id: &ProfileId) -> AsyncFallible<Vec<Link>> {
        unimplemented!()
        // Ok( Vec::new() ) // TODO implement this properly
    }
}

impl Home for HomeClientCapnProto {
    fn claim(&self, profile_id: ProfileId) -> AsyncResult<OwnProfile, Error> {
        let mut request = self.home.claim_request();
        request.get().set_profile_id(&profile_id.to_bytes());

        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| {
                resp.get()
                    .and_then(|res| res.get_own_profile())
                    .and_then(|own_prof_capnp| OwnProfile::try_from(own_prof_capnp))
            })
            .map_err(|e| e.context(ErrorKind::FailedToClaimProfile).into());

        Box::new(resp_fut)
    }

    fn register(
        &self,
        own_profile: OwnProfile,
        half_proof: RelationHalfProof,
        //invite: Option<HomeInvitation>,
    ) -> AsyncResult<OwnProfile, (OwnProfile, Error)> {
        let mut request = self.home.register_request();
        request.get().init_own_profile().fill_from(&own_profile);
        request.get().init_half_proof().fill_from(&half_proof);
        //if let Some(inv) = invite {
        //    request.get().init_invite().fill_from(&inv);
        //}

        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| {
                resp.get()
                    .and_then(|res| res.get_own_profile())
                    .and_then(|own_prof_capnp| OwnProfile::try_from(own_prof_capnp))
            })
            .map_err(move |e| (own_profile, e.context(ErrorKind::RegisterFailed).into()));

        Box::new(resp_fut)
    }

    fn login(&self, proof_of_home: &RelationProof) -> AsyncResult<Rc<HomeSession>, Error> {
        let mut request = self.home.login_request();
        request.get().init_proof_of_home().fill_from(proof_of_home);

        let handle_clone = self.handle.clone();
        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| {
                resp.get().and_then(|res| res.get_session()).map(|session_client| {
                    Rc::new(HomeSessionClientCapnProto::new(session_client, handle_clone))
                        as Rc<HomeSession>
                })
            })
            .map_err(|e| e.context(ErrorKind::FailedToCreateSession).into());

        Box::new(resp_fut)
    }

    // NOTE acceptor must have this server as its home
    fn pair_request(&self, half_proof: RelationHalfProof) -> AsyncResult<(), Error> {
        let mut request = self.home.pair_request_request();
        request.get().init_half_proof().fill_from(&half_proof);

        let resp_fut = request
            .send()
            .promise
            .map(|_resp| ())
            .map_err(|e| e.context(ErrorKind::PairRequestFailed).into());

        Box::new(resp_fut)
    }

    // NOTE acceptor must have this server as its home
    fn pair_response(&self, relation_proof: RelationProof) -> AsyncResult<(), Error> {
        let mut request = self.home.pair_response_request();
        request.get().init_relation().fill_from(&relation_proof);

        let resp_fut = request
            .send()
            .promise
            .map(|_resp| ())
            .map_err(|e| e.context(ErrorKind::PairResponseFailed).into());

        Box::new(resp_fut)
    }

    fn call(
        &self,
        app: ApplicationId,
        call_req: CallRequestDetails,
    ) -> AsyncResult<Option<AppMsgSink>, Error> {
        let mut request = self.home.call_request();
        request.get().init_relation().fill_from(&call_req.relation);
        request.get().set_app((&app).into());
        request.get().set_init_payload((&call_req.init_payload).into());

        if let Some(send) = call_req.to_caller {
            let to_caller_dispatch = mercury_capnp::AppMessageDispatcherCapnProto::new(send);
            let to_caller_capnp =
                mercury_capnp::app_message_listener::ToClient::new(to_caller_dispatch)
                    .into_client::<::capnp_rpc::Server>();
            request.get().set_to_caller(to_caller_capnp);
        }

        let handle_clone = self.handle.clone();
        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| {
                resp.get().map(|res| {
                    res.get_to_callee()
                        .map(|to_callee_capnp| {
                            mercury_capnp::fwd_appmsg(to_callee_capnp, handle_clone)
                        })
                        .ok()
                })
            })
            .map_err(|e| e.context(ErrorKind::CallFailed).into());

        Box::new(resp_fut)
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
        let event_capnp = pry!(pry!(params.get()).get_event());
        let event = pry!(ProfileEvent::try_from(event_capnp));
        trace!("Capnp client received event: {:?}", event);
        let recv_fut = self
            .sender
            .clone()
            .send(Ok(event))
            .map(|_sink| ())
            .map_err(|e| capnp::Error::failed(format!("Failed to delegate event: {}", e)));
        Promise::from_future(recv_fut)
    }

    fn error(
        &mut self,
        params: mercury_capnp::profile_event_listener::ErrorParams,
        _results: mercury_capnp::profile_event_listener::ErrorResults,
    ) -> Promise<(), capnp::Error> {
        let error = pry!(pry!(params.get()).get_error()).into();
        let recv_fut =
            self.sender.clone().send(Err(error)).map(|_sink| ()).map_err(|e| {
                capnp::Error::failed(format!("Failed to delegate event error: {}", e))
            });
        Promise::from_future(recv_fut)
    }
}

pub struct HomeSessionClientCapnProto {
    session: mercury_capnp::home_session::Client,
    handle: reactor::Handle,
}

impl HomeSessionClientCapnProto {
    pub fn new(session: mercury_capnp::home_session::Client, handle: reactor::Handle) -> Self {
        Self { session, handle }
    }
}

impl HomeSession for HomeSessionClientCapnProto {
    // TODO consider if we should notify an open session about an updated profile
    // TODO consider if an OwnProfile return value is needed or how to force updating
    //      the currently active profile in all PeerContext/Session/etc instances
    fn update(&self, own_prof: OwnProfile) -> AsyncResult<(), Error> {
        let mut request = self.session.update_request();
        request.get().init_own_profile().fill_from(&own_prof);

        let resp_fut = request
            .send()
            .promise
            .map(|_resp| ())
            .map_err(|e| e.context(ErrorKind::ProfileUpdateFailed).into());

        Box::new(resp_fut)
    }

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    fn unregister(&self, newhome: Option<Profile>) -> AsyncResult<(), Error> {
        let mut request = self.session.unregister_request();
        if let Some(new_home_profile) = newhome {
            request.get().set_new_home(&profile_to_bytes(&new_home_profile));
        }

        let resp_fut = request
            .send()
            .promise
            .map(|_resp| ())
            .map_err(|e| e.context(ErrorKind::UnregisterFailed).into());

        Box::new(resp_fut)
    }

    fn events(&self) -> AsyncStream<ProfileEvent, String> {
        let (send, recv) = mpsc::channel(1);
        let listener = ProfileEventDispatcherCapnProto::new(send.clone());
        // TODO consider how to drop/unregister this object from capnp if the stream is dropped
        let listener_capnp = mercury_capnp::profile_event_listener::ToClient::new(listener)
            .into_client::<::capnp_rpc::Server>();

        let mut request = self.session.events_request();
        request.get().set_event_listener(listener_capnp);

        self.handle.spawn(request.send().promise.map(|_resp| ()).or_else(move |e| {
            send.send(Err(format!("Events delegation failed: {}", e)))
                    .map(|_sink| ())
                    // TODO what to do if failed to send error?
                    .map_err(|_err| ())
        }));

        recv
    }

    fn checkin_app(&self, app: &ApplicationId) -> AsyncStream<Box<IncomingCall>, String> {
        // Send a call dispatcher proxy to remote home through which we'll accept incoming calls
        let (send, recv) = mpsc::channel(1);
        let listener = CallDispatcherCapnProto::new(send.clone(), self.handle.clone());
        // TODO consider how to drop/unregister this object from capnp if the stream is dropped
        let listener_capnp = mercury_capnp::call_listener::ToClient::new(listener)
            .into_client::<::capnp_rpc::Server>();

        let mut request = self.session.checkin_app_request();
        request.get().set_app(app.into());
        request.get().set_call_listener(listener_capnp);

        // We can either return Future<Stream> or
        // return the stream directly and spawn sending the request in another fiber
        self.handle.spawn(request.send().promise.map(|_resp| ()).or_else(move |e| {
            send.send(Err(format!("Call delegation failed: {}", e)))
                    .map(|_sink| ())
                    // TODO what to do if failed to send error?
                    .map_err(|_err| ())
        }));

        recv
    }

    fn ping(&self, txt: &str) -> AsyncResult<String, Error> {
        let mut request = self.session.ping_request();
        request.get().set_txt(txt);

        let resp_fut = request
            .send()
            .promise
            .and_then(|resp| resp.get().and_then(|res| res.get_pong()).map(|pong| pong.to_owned()))
            .map_err(|e| e.context(ErrorKind::PingFailed).into());

        Box::new(resp_fut)
    }
}

const CALL_TIMEOUT_SECS: u32 = 30;

struct CallDispatcherCapnProto {
    sender: mpsc::Sender<Result<Box<IncomingCall>, String>>,
    handle: reactor::Handle,
}

impl CallDispatcherCapnProto {
    fn new(
        sender: mpsc::Sender<Result<Box<IncomingCall>, String>>,
        handle: reactor::Handle,
    ) -> Self {
        Self { sender, handle }
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
        // NOTE there's no way to add the i/o streams in try_from without extra context,
        //      we have to set them manually
        let call_capnp = pry!(pry!(params.get()).get_call());
        let mut call = pry!(CallRequestDetails::try_from(call_capnp));

        // If received a to_caller channel, setup an in-memory sink for easier sending
        call.to_caller = call_capnp
            .get_to_caller()
            .map(|to_caller_capnp| mercury_capnp::fwd_appmsg(to_caller_capnp, self.handle.clone()))
            .ok();

        let (one_send, one_recv) = oneshot::channel();
        let answer_fut = one_recv
            .map(|to_callee_opt: Option<AppMsgSink>| {
                // If the call is accepted then set up a to_callee channel and send it back in the response
                to_callee_opt.map(move |to_callee| {
                    let listener = mercury_capnp::AppMessageDispatcherCapnProto::new(to_callee);
                    // TODO consider how to drop/unregister this object from capnp if the stream is dropped
                    let listener_capnp =
                        mercury_capnp::app_message_listener::ToClient::new(listener)
                            .into_client::<::capnp_rpc::Server>();
                    results.get().set_to_callee(listener_capnp);
                });
            })
            .map_err(|e| {
                capnp::Error::failed(format!("Failed to get answer from callee: {:?}", e))
            }); // TODO should we send an error back to the caller?

        // TODO make this timeout period user-configurable
        let timeout_res =
            reactor::Timeout::new(Duration::from_secs(CALL_TIMEOUT_SECS.into()), &self.handle);
        let timeout_fut = pry!(timeout_res)
            .map_err(|e| capnp::Error::failed(format!("Call timed out without answer: {:?}", e))); // TODO should we send an error back to the caller?

        // Call will time out if not answered in a given period
        let answer_or_timeout_fut = answer_fut
            .select(timeout_fut)
            .map(|(completed_item, _pending_fut)| completed_item)
            .map_err(|(completed_err, _pending_err)| completed_err);

        // Set up an IncomingCall object allowing to decide answering or refusing the call
        // TODO consider error handling: should we send error and close the sink in case of errors above?
        let incoming_call = Box::new(IncomingCallCapnProto::new(call, one_send));
        let call_fut = self
            .sender
            .clone()
            .send(Ok(incoming_call))
            .map(|_sink| ())
            .map_err(|e| capnp::Error::failed(format!("Failed to dispatch call: {:?}", e))) // TODO should we send an error back to the caller?
            // and require the call to be answered or dropped
            .and_then(|()| answer_or_timeout_fut);

        // TODO consider if the call (e.g. channels and capnp server objects) is dropped after a timeout
        //      but lives after properly accepted
        Promise::from_future(call_fut)
    }

    fn error(
        &mut self,
        params: mercury_capnp::call_listener::ErrorParams,
        _results: mercury_capnp::call_listener::ErrorResults,
    ) -> Promise<(), capnp::Error> {
        let error = pry!(pry!(params.get()).get_error()).into();
        let recv_fut = self
            .sender
            .clone()
            .send(Err(error))
            .map(|_sink| ())
            .map_err(|e| capnp::Error::failed(format!("Failed to dispatch call error: {}", e)));
        Promise::from_future(recv_fut)
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
