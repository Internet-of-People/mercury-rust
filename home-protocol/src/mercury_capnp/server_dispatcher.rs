use std::convert::TryFrom;
use std::rc::Rc;

use capnp::capability::Promise;
use futures::io::{AsyncRead, AsyncWrite};
use tokio::net::tcp::TcpStream;
use tokio::runtime::current_thread as reactor;

use super::*;
use crate::mercury_capnp::FillFrom;

// TODO Rename to HomeDispatcherCapnp in accordance with lots of names around
pub struct HomeDispatcherCapnProto {
    home: Rc<dyn Home>,
    // TODO probably we should have a SessionFactory here instead of instantiating sessions "manually"
}

impl HomeDispatcherCapnProto {
    // TODO how to access PeerContext in the Home implementation?
    pub fn dispatch<R, W>(home: Rc<dyn Home>, reader: R, writer: W)
    where
        R: AsyncRead + Unpin + 'static,
        W: AsyncWrite + Unpin + 'static,
    {
        let f = async move {
            let dispatcher = Self { home };

            let home_capnp =
                mercury_capnp::home::ToClient::new(dispatcher).into_client::<::capnp_rpc::Server>();
            let network = capnp_rpc::twoparty::VatNetwork::new(
                reader,
                writer,
                capnp_rpc::rpc_twoparty_capnp::Side::Server,
                Default::default(),
            );

            let rpc_system = capnp_rpc::RpcSystem::new(Box::new(network), Some(home_capnp.client));

            match rpc_system.await {
                Ok(()) => info!("Capnp RPC finished"),
                Err(e) => warn!("Capnp RPC failed: {}", e),
            };
        };
        reactor::spawn(f);
    }

    pub fn dispatch_tcp(home: Rc<dyn Home>, tcp_stream: TcpStream) {
        tcp_stream.set_nodelay(true).unwrap();
        let x = futures_tokio_compat::Compat::new(tcp_stream);
        let (reader, writer) = x.split();
        HomeDispatcherCapnProto::dispatch(home, reader, writer)
    }
}

// NOTE useful for testing connection lifecycles
impl Drop for HomeDispatcherCapnProto {
    fn drop(&mut self) {
        debug!("Home connection dropped");
    }
}

impl mercury_capnp::profile_repo::Server for HomeDispatcherCapnProto {
    fn get(
        &mut self,
        params: mercury_capnp::profile_repo::GetParams,
        mut results: mercury_capnp::profile_repo::GetResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let profile_id_capnp = params.get()?.get_profile_id()?;
            let profile_id =
                ProfileId::from_bytes(profile_id_capnp).map_err_capnp("Parsing profile id")?;

            let profile = home.fetch(&profile_id).await.map_err_capnp("Failed to load profile")?;

            results.get().set_profile(&profile_to_bytes(&profile));
            Ok(())
        };
        Promise::from_future(f)
    }
}

impl mercury_capnp::home::Server for HomeDispatcherCapnProto {
    fn claim(
        &mut self,
        params: mercury_capnp::home::ClaimParams,
        mut results: mercury_capnp::home::ClaimResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let profile_id_capnp = params.get()?.get_profile_id()?;
            let profile_id =
                ProfileId::from_bytes(profile_id_capnp).map_err_capnp("Parsing profile id")?;

            let hosting_proof =
                home.claim(profile_id).await.map_err_capnp("Failed to claim profile")?;

            results.get().init_hosting_proof().fill_from(&hosting_proof);
            Ok(())
        };
        Promise::from_future(f)
    }

    fn register(
        &mut self,
        params: mercury_capnp::home::RegisterParams,
        mut results: mercury_capnp::home::RegisterResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let half_proof_capnp = params.get()?.get_half_proof()?;
            let half_proof = RelationHalfProof::try_from(half_proof_capnp)?;
            // let inv_capnp_opt = params.get()?.get_invite().ok();
            // let invite_opt: Option<HomeInvitation> = inv_capnp_opt.map(HomeInvitation::try_from);

            let proof = home
                .register(&half_proof /*, invite_opt*/)
                .await
                .map_err_capnp("Failed to register profile")?;

            results.get().init_hosting_proof().fill_from(&proof);
            Ok(())
        };
        Promise::from_future(f)
    }

    fn login(
        &mut self,
        params: mercury_capnp::home::LoginParams,
        mut results: mercury_capnp::home::LoginResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let host_proof_capnp = params.get()?.get_hosting_proof()?;
            let host_proof = RelationProof::try_from(host_proof_capnp)?;
            let session_impl = home.login(&host_proof).await.map_err_capnp("Failed to login")?;

            let session_dispatcher = HomeSessionDispatcherCapnProto::new(session_impl);
            let session = mercury_capnp::home_session::ToClient::new(session_dispatcher)
                .into_client::<capnp_rpc::Server>();
            results.get().set_session(session);
            Ok(())
        };
        Promise::from_future(f)
    }

    fn pair_request(
        &mut self,
        params: mercury_capnp::home::PairRequestParams,
        mut _results: mercury_capnp::home::PairRequestResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let half_proof_capnp = params.get()?.get_half_proof()?;
            let half_proof = RelationHalfProof::try_from(half_proof_capnp)?;

            home.pair_request(&half_proof).await.map_err_capnp("Failed to request pairing")?;

            Ok(())
        };
        Promise::from_future(f)
    }

    fn pair_response(
        &mut self,
        params: mercury_capnp::home::PairResponseParams,
        mut _results: mercury_capnp::home::PairResponseResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let proof_capnp = params.get()?.get_relation()?;
            let proof = RelationProof::try_from(proof_capnp)?;

            home.pair_response(&proof).await.map_err_capnp("Failed to send pairing response")?;

            Ok(())
        };
        Promise::from_future(f)
    }

    fn call(
        &mut self,
        params: mercury_capnp::home::CallParams,
        mut results: mercury_capnp::home::CallResults,
    ) -> Promise<(), capnp::Error> {
        let home = self.home.to_owned();
        let f = async move {
            let opts = params.get()?;

            let rel_capnp = opts.get_relation()?;
            let app_capnp = opts.get_app()?;
            let init_payload_capnp = opts.get_init_payload()?;
            let to_caller_opt_capnp = opts.get_to_caller().ok();

            let relation = RelationProof::try_from(rel_capnp)?;
            let app = ApplicationId::from(app_capnp);
            let init_payload = AppMessageFrame::from(init_payload_capnp);
            let to_caller = to_caller_opt_capnp.map(AppMsgSink::from);

            let call_req = CallRequestDetails { relation, init_payload, to_caller };
            let to_callee_opt =
                home.call(app, &call_req).await.map_err_capnp("Failed to call profile")?;

            to_callee_opt.map(|sink| results.get().set_to_callee(sink.into()));
            Ok(())
        };
        Promise::from_future(f)
    }
}

pub struct HomeSessionDispatcherCapnProto {
    session: Rc<dyn HomeSession>,
}

impl HomeSessionDispatcherCapnProto {
    pub fn new(session: Rc<dyn HomeSession>) -> Self {
        Self { session }
    }
}

// NOTE useful for testing connection lifecycles
impl Drop for HomeSessionDispatcherCapnProto {
    fn drop(&mut self) {
        debug!("Session over Home connection dropped");
    }
}

impl mercury_capnp::home_session::Server for HomeSessionDispatcherCapnProto {
    fn backup(
        &mut self,
        params: mercury_capnp::home_session::BackupParams,
        mut _results: mercury_capnp::home_session::BackupResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();
        let f = async move {
            let own_profile_capnp = params.get()?.get_own_profile()?;
            let own_profile = bytes_to_own_profile(own_profile_capnp)?;

            session.backup(own_profile).await.map_err_capnp("Failed to update profile")?;
            Ok(())
        };
        Promise::from_future(f)
    }

    fn restore(
        &mut self,
        _params: mercury_capnp::home_session::RestoreParams,
        mut results: mercury_capnp::home_session::RestoreResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();
        let f = async move {
            let own_prof = session.restore().await.map_err_capnp("Failed to restore profile")?;

            let own_bytes = own_profile_to_bytes(&own_prof);
            results.get().set_own_profile(&own_bytes);
            Ok(())
        };
        Promise::from_future(f)
    }

    fn unregister(
        &mut self,
        params: mercury_capnp::home_session::UnregisterParams,
        mut _results: mercury_capnp::home_session::UnregisterResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();
        let f = async move {
            let new_home_capnp_opt = params.get()?.get_new_home().ok();
            let new_home_opt = if let Some(new_home_capnp) = new_home_capnp_opt {
                Some(bytes_to_profile(&new_home_capnp).map_err_capnp("Parsing profile")?)
            } else {
                None
            };

            session.unregister(new_home_opt).await.map_err_capnp("Failed to unregister profile")?;
            Ok(())
        };
        Promise::from_future(f)
    }

    fn ping(
        &mut self,
        params: mercury_capnp::home_session::PingParams,
        mut results: mercury_capnp::home_session::PingResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();
        let f = async move {
            let txt = params.get()?.get_txt()?;

            let pong = session.ping(txt).await.map_err_capnp("Failed ping")?;

            results.get().set_pong(&pong);
            capnp::Result::Ok(())
        };
        Promise::from_future(f)
    }

    fn events(
        &mut self,
        params: mercury_capnp::home_session::EventsParams,
        mut _results: mercury_capnp::home_session::EventsResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();

        async fn handle_profile_event(
            callback: &mercury_capnp::profile_event_listener::Client,
            item: Result<ProfileEvent, String>,
        ) -> () {
            debug!("Capnp server is forwarding event to the client: {:?}", item);
            match item {
                Ok(event) => {
                    let mut request = callback.receive_request();
                    request.get().init_event().fill_from(&event);

                    let _res = request.send().promise.await;
                }
                Err(err) => {
                    let mut request = callback.error_request();
                    request.get().set_error(&err);

                    let _res = request.send().promise.await;
                }
            }
        }

        let f = async move {
            let callback = params.get()?.get_event_listener()?;
            let mut event_stream = session.events();
            while let Some(item) = event_stream.next().await {
                handle_profile_event(&callback, item).await;
            }
            capnp::Result::Ok(())
        };
        Promise::from_future(f)
    }

    fn checkin_app(
        &mut self,
        params: mercury_capnp::home_session::CheckinAppParams,
        _results: mercury_capnp::home_session::CheckinAppResults,
    ) -> Promise<(), capnp::Error> {
        let session = self.session.to_owned();

        async fn handle_call(
            callback: &call_listener::Client,
            item: Result<Box<dyn IncomingCall>, String>,
        ) -> () {
            debug!("Capnp server is forwarding a call to the client");
            match item {
                Ok(incoming_call) => {
                    let mut request = callback.receive_request();
                    let details = incoming_call.request_details();
                    request.get().init_call().fill_from(details);
                    if let Some(to_caller) = details.to_caller.to_owned() {
                        request.get().get_call().expect("Implementation error: call was just initialized above, should be there")
                            .set_to_caller(to_caller.into());
                    }

                    if let Ok(resp) = request.send().promise.await {
                        let scope = || resp.get()?.get_to_callee();
                        if let Ok(to_callee_capnp) = scope() {
                            let answer = mercury_capnp::fwd_appmsg(to_callee_capnp);
                            incoming_call.answer(Some(answer));
                        }
                    }
                }
                Err(err) => {
                    let mut request = callback.error_request();
                    request.get().set_error(&err);

                    let _res = request.send().promise.await;
                }
            }
        }

        let f = async move {
            // Receive a proxy from client to which the server will send notifications on incoming calls
            let opts = params.get()?;
            let app_id = opts.get_app()?;
            let call_listener = opts.get_call_listener()?;

            // Forward incoming calls from business logic into capnp proxy stub of client
            let mut call_stream = session.checkin_app(&app_id.into());
            while let Some(item) = call_stream.next().await {
                handle_call(&call_listener, item).await;
            }
            capnp::Result::Ok(())
        };
        Promise::from_future(f)
    }
}
