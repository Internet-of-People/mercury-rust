include!(concat!(env!("OUT_DIR"), "/protocol/mercury_capnp.rs"));

pub mod client_proxy;
pub mod server_dispatcher;

use std::convert::TryFrom;

use capnp;
use capnp::capability::Promise;
use futures::prelude::*;
use tokio::sync::mpsc;

use crate::*;

trait ResultCapnpExt<T> {
    fn map_err_capnp(self, msg: &str) -> capnp::Result<T>;
}

//Failed to send event
impl<T, E: std::fmt::Debug> ResultCapnpExt<T> for Result<T, E> {
    fn map_err_capnp(self, msg: &str) -> capnp::Result<T> {
        self.map_err(|e| capnp::Error::failed(format!("{}: {:?}", msg, e)))
    }
}

pub trait PromiseUtil<T, E> {
    fn result(result: Result<T, E>) -> Promise<T, E>
    where
        T: 'static,
        E: 'static,
    {
        match result {
            Ok(t) => Promise::ok(t),
            Err(e) => Promise::err(e),
        }
    }
}

impl<T, E> PromiseUtil<T, E> for Promise<T, E> {}

pub trait FillFrom<T> {
    fn fill_from(self, source: T);
}

impl<'a> From<&'a [u8]> for AppMessageFrame {
    fn from(src: &'a [u8]) -> Self {
        AppMessageFrame(src.to_owned())
    }
}

impl<'a> From<&'a AppMessageFrame> for &'a [u8] {
    fn from(src: &'a AppMessageFrame) -> Self {
        &src.0
    }
}

impl<'a> From<&'a str> for ApplicationId {
    fn from(src: &'a str) -> Self {
        ApplicationId(src.to_owned())
    }
}

impl<'a> From<&'a ApplicationId> for &'a str {
    fn from(src: &'a ApplicationId) -> Self {
        &src.0
    }
}

fn bytes_to_profile(src: &[u8]) -> serde_json::Result<Profile> {
    serde_json::from_slice(&src)
}

fn profile_to_bytes(src: &Profile) -> Vec<u8> {
    // TODO how to return error here without changing the signature of fill_from()?
    serde_json::to_vec(src)
        .expect("Implementation error: serialization can fail only if Serialize implementation returns error or with non-string keys in the type")
}

fn bytes_to_own_profile(src: &[u8]) -> Result<OwnProfile, capnp::Error> {
    serde_json::from_slice(&src).map_err(|e| capnp::Error::failed(e.to_string()))
}

fn own_profile_to_bytes(src: &OwnProfile) -> Vec<u8> {
    // TODO how to return error here without changing the signature of fill_from()?
    serde_json::to_vec(src)
        .expect("Implementation error: serialization can fail only if Serialize implementation returns error or with non-string keys in the type")
}

impl<'a> TryFrom<relation_half_proof::Reader<'a>> for RelationHalfProof {
    type Error = capnp::Error;

    fn try_from(src: relation_half_proof::Reader) -> Result<Self, Self::Error> {
        Ok(RelationHalfProof {
            relation_type: String::from(src.get_relation_type()?),
            signer_id: ProfileId::from_bytes(src.get_signer_id()?)
                .map_err_capnp("Parsing signer id")?,
            signer_pubkey: PublicKey::from_bytes(src.get_signer_pub_key()?)
                .map_err_capnp("Parsing signer pubkey")?,
            peer_id: ProfileId::from_bytes(src.get_peer_id()?).map_err_capnp("Parsing peer id")?,
            signature: Signature::from_bytes(src.get_signature()?)
                .map_err_capnp("Parsing signature")?,
        })
    }
}

impl<'a> FillFrom<&RelationHalfProof> for relation_half_proof::Builder<'a> {
    fn fill_from(mut self, src: &RelationHalfProof) {
        self.set_relation_type(&src.relation_type);
        self.set_signer_id(&src.signer_id.to_bytes());
        self.set_signer_pub_key(&src.signer_pubkey.to_bytes());
        self.set_peer_id(&src.peer_id.to_bytes());
        self.set_signature(&src.signature.to_bytes());
    }
}

impl<'a> TryFrom<relation_proof::Reader<'a>> for RelationProof {
    type Error = capnp::Error;

    fn try_from(src: relation_proof::Reader) -> Result<Self, Self::Error> {
        Ok(RelationProof {
            relation_type: String::from(src.get_relation_type()?),
            a_id: ProfileId::from_bytes(src.get_a_id()?).map_err_capnp("Parsing id A")?,
            a_pub_key: PublicKey::from_bytes(src.get_a_pub_key()?)
                .map_err_capnp("Parsing pubkey A")?,
            a_signature: Signature::from_bytes(src.get_a_signature()?)
                .map_err_capnp("Parsing signature A")?,
            b_id: ProfileId::from_bytes(src.get_b_id()?).map_err_capnp("Parsing id B")?,
            b_pub_key: PublicKey::from_bytes(src.get_b_pub_key()?)
                .map_err_capnp("Parsing pubkey B")?,
            b_signature: Signature::from_bytes(src.get_b_signature()?)
                .map_err_capnp("Parsing signature B")?,
        })
    }
}

impl<'a> FillFrom<&RelationProof> for relation_proof::Builder<'a> {
    fn fill_from(mut self, src: &RelationProof) {
        self.set_relation_type(&src.relation_type);
        self.set_a_id(&src.a_id.to_bytes());
        self.set_a_pub_key(&src.a_pub_key.to_bytes());
        self.set_a_signature(&src.a_signature.to_bytes());
        self.set_b_id(&src.b_id.to_bytes());
        self.set_b_pub_key(&src.b_pub_key.to_bytes());
        self.set_b_signature(&src.b_signature.to_bytes());
    }
}

impl<'a> TryFrom<profile_event::Reader<'a>> for ProfileEvent {
    type Error = capnp::Error;

    fn try_from(src: profile_event::Reader) -> Result<Self, Self::Error> {
        match src.which()? {
            profile_event::Which::Unknown(data) => Ok(ProfileEvent::Unknown(Vec::from(data?))),
            profile_event::Which::PairingRequest(half_proof) => {
                Ok(ProfileEvent::PairingRequest(RelationHalfProof::try_from(half_proof?)?))
            }
            profile_event::Which::PairingResponse(proof) => {
                Ok(ProfileEvent::PairingResponse(RelationProof::try_from(proof?)?))
            }
        }
    }
}

impl<'a> FillFrom<&ProfileEvent> for profile_event::Builder<'a> {
    fn fill_from(self, src: &ProfileEvent) {
        match src {
            ProfileEvent::PairingRequest(half_proof) => {
                let mut builder = self.init_pairing_request();
                builder.reborrow().fill_from(half_proof);
            }
            ProfileEvent::PairingResponse(proof) => {
                let mut builder = self.init_pairing_response();
                builder.reborrow().fill_from(proof);
            }
            ProfileEvent::Unknown(data) => {
                let _builder = self.init_unknown(data.len() as u32);
                // TODO fill with data
                // builder.reborrow().fill_with(data);
            }
        };
    }
}

impl<'a> TryFrom<call_request::Reader<'a>> for CallRequestDetails {
    type Error = capnp::Error;

    fn try_from(src: call_request::Reader) -> Result<Self, Self::Error> {
        let relation = RelationProof::try_from(src.get_relation()?)?;
        let init_payload = src.get_init_payload()?.into();

        Ok(CallRequestDetails { relation, init_payload, to_caller: None })
    }
}

impl<'a> FillFrom<&CallRequestDetails> for call_request::Builder<'a> {
    fn fill_from(mut self, src: &CallRequestDetails) {
        self.set_init_payload((&src.init_payload).into());
        self.init_relation().fill_from(&src.relation);
        // TODO set up channel to caller: is it possible here without external context?
        // self.set_to_caller( TODO );
    }
}

// TODO consider using a single generic implementation for all kinds of Dispatchers
pub struct AppMessageDispatcherCapnProto {
    sender: AppMsgSink,
}

impl AppMessageDispatcherCapnProto {
    pub fn new(sender: AppMsgSink) -> Self {
        Self { sender }
    }
}

impl app_message_listener::Server for AppMessageDispatcherCapnProto {
    fn receive(
        &mut self,
        params: app_message_listener::ReceiveParams,
        _results: app_message_listener::ReceiveResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.clone();
        let f = async move {
            let message = params.get()?.get_message()?;
            sender.send(Ok(message.into())).await.map_err_capnp("Failed to send event")
        };
        Promise::from_future(f)
    }

    fn error(
        &mut self,
        params: app_message_listener::ErrorParams,
        _results: app_message_listener::ErrorResults,
    ) -> Promise<(), capnp::Error> {
        let mut sender = self.sender.clone();
        let f = async move {
            let error = params.get()?.get_error()?;
            sender.send(Err(error.into())).await.map_err_capnp("Failed to send error")
        };
        Promise::from_future(f)
    }
}

impl From<AppMsgSink> for app_message_listener::Client {
    fn from(to_callee: AppMsgSink) -> Self {
        let to_callee_dispatch = mercury_capnp::AppMessageDispatcherCapnProto::new(to_callee);
        let to_callee_capnp =
            mercury_capnp::app_message_listener::ToClient::new(to_callee_dispatch)
                .into_client::<::capnp_rpc::Server>();
        to_callee_capnp
    }
}

impl From<app_message_listener::Client> for AppMsgSink {
    fn from(client: app_message_listener::Client) -> Self {
        fwd_appmsg(client)
    }
}

pub fn fwd_appmsg(to_callee: app_message_listener::Client) -> AppMsgSink {
    let (send, mut recv) = mpsc::channel::<Result<AppMessageFrame, String>>(1);

    async fn handle_message(
        to_callee: &app_message_listener::Client,
        message: Result<AppMessageFrame, String>,
    ) -> () {
        let _res = match message {
            Ok(msg) => {
                let mut request = to_callee.receive_request();
                request.get().set_message(&msg.0);
                request.send().promise.await.map(|_r| ())
            }
            Err(err) => {
                let mut request = to_callee.error_request();
                request.get().set_error(&err);
                request.send().promise.await.map(|_r| ())
            }
        };
        // TODO what to do here with the network capnp error?
    }

    let f = async move {
        while let Some(message) = recv.next().await {
            handle_message(&to_callee, message).await;
        }
    };
    tokio::runtime::current_thread::spawn(f);

    send
}

#[cfg(test)]
mod tests {}
