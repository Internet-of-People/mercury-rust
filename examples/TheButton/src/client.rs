use futures::prelude::*;
use log::*;

use crate::init_hack::init_app_common;
use crate::*;
use mercury_home_protocol::*;

#[derive(Clone)]
pub struct Client {
    pub cfg: ClientConfig,
    pub appctx: AppContext,
}

impl Client {
    pub fn new(cfg: ClientConfig, appctx: AppContext) -> Self {
        Self { appctx, cfg }
    }

    pub fn wait_for_pairing_response(
        events: Box<Stream<Item = DAppEvent, Error = ()>>,
        my_profile_id: ProfileId,
        handle: reactor::Handle,
    ) -> AsyncResult<Box<Contact>, Error> {
        let fut = events
            .filter_map(move |event| {
                debug!("TheButton got event");
                if let DAppEvent::PairingResponse(relation) = event {
                    trace!(
                        "Got pairing response, checking peer id: {:?}",
                        relation.proof()
                    );
                    if relation.proof().peer_id(&my_profile_id).is_ok() {
                        return Some(relation);
                    }
                }
                return None;
            })
            .take(1)
            .into_future() // NOTE transforms stream into a future of an (item,stream) pair
            .map_err(|((), _stream)| {
                debug!("Pairing failed");
                Error::from(ErrorKind::LookupFailed)
            })
            .and_then(|(proof, _stream)| {
                proof.ok_or_else(|| {
                    debug!("Profile event stream ended without proper response");
                    Error::from(ErrorKind::LookupFailed)
                })
            })
            .and_then(move |proof| {
                reactor::Timeout::new(std::time::Duration::from_millis(10), &handle)
                    .unwrap()
                    .map(|_| proof)
                    .map_err(|e| e.context(ErrorKind::ImplementationError).into())
            });
        Box::new(fut)
    }

    fn get_or_create_contact(
        self,
        dapp_session: Rc<DAppSession>,
    ) -> AsyncResult<Box<Contact>, Error> {
        let callee_profile_id = self.cfg.callee_profile_id.clone();
        let contact_fut = dapp_session
            .contacts_with_profile(&callee_profile_id, None)
            .map(|relations| (dapp_session, relations))
            .and_then({
                let peer_id = self.cfg.callee_profile_id.clone();
                let client_id = self.appctx.client_id.clone();
                let handle = self.appctx.handle.clone();
                move |(dapp_session, mut relations)| {
                    let init_rel_fut = dapp_session.initiate_contact(&peer_id);
                    match relations.pop() {
                        Some(relation) => Box::new(Ok(relation).into_future()) as AsyncResult<_, _>,
                        None => {
                            let rel_fut = dapp_session
                                .checkin()
                                .and_then(|events| init_rel_fut.map(|()| events))
                                .and_then(|events| {
                                    Self::wait_for_pairing_response(events, client_id, handle)
                                });
                            Box::new(rel_fut)
                        }
                    }
                }
            });
        Box::new(contact_fut)
    }
}

impl IntoFuture for Client {
    type Future = AsyncResult<Self::Item, Self::Error>;
    type Item = ();
    type Error = Error;

    fn into_future(self) -> Self::Future {
        let client_fut = self
            .appctx
            .service
            .dapp_session(&self.appctx.app_id, None)
            .and_then({
                let client = self.clone();
                move |dapp_session| client.get_or_create_contact(dapp_session)
            })
            .and_then(|contact| {
                info!("Contact is available, start calling");
                contact.call(AppMessageFrame(vec![])).map_err(|err| {
                    error!("call failed: {:?}", err);
                    err
                })
            })
            .and_then(|call| {
                info!("call accepted, waiting for incoming messages");
                call.incoming
                    .for_each(|msg: Result<AppMessageFrame, String>| {
                        msg.map(|frame| info!("Client received server message {:?}", frame))
                            .map_err(|err| warn!("Client got server error {:?}", err))
                    })
                    .map_err(|()| Error::from(ErrorKind::CallFailed))
            });

        Box::new(init_app_common(&self.appctx).then(|_res| client_fut))
    }
}
