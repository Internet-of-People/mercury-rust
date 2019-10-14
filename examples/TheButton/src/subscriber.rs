use futures::prelude::*;
use log::*;

use crate::options::SubscriberConfig;
use crate::*;
use mercury_home_protocol::*;

#[derive(Clone)]
pub struct Client {
    pub cfg: SubscriberConfig,
    pub appctx: AppContext,
}

impl Client {
    pub fn new(cfg: SubscriberConfig, appctx: AppContext) -> Self {
        Self { appctx, cfg }
    }

    pub fn wait_for_pairing_response(
        events: Box<dyn Stream<Item = DAppEvent, Error = ()>>,
        my_profile_id: ProfileId,
    ) -> AsyncFallible<Box<dyn Relation>> {
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
                err_msg("Pairing failed")
            })
            .and_then(|(proof, _stream)| {
                proof.ok_or_else(|| {
                    debug!("Profile event stream ended without proper response");
                    err_msg("Profile event stream ended without proper response")
                })
            });
        Box::new(fut)
    }

    fn get_or_create_contact(
        self,
        dapp_session: Rc<dyn DAppSession>,
    ) -> AsyncFallible<Box<dyn Relation>> {
        let callee_profile_id = self.cfg.server_id.clone();
        let contact_fut = dapp_session.relation(&callee_profile_id).and_then({
            let peer_id = self.cfg.server_id.clone();
            move |relation| {
                let init_rel_fut = dapp_session.initiate_relation(&peer_id);
                match relation {
                    Some(relation) => Box::new(Ok(relation).into_future()) as AsyncResult<_, _>,
                    None => {
                        debug!("No signed relation to server is available, initiate pairing");
                        let persona_id = dapp_session.selected_profile().to_owned();
                        let rel_fut = dapp_session
                            .checkin()
                            .and_then(|events| init_rel_fut.map(|()| events))
                            .and_then(|events| {
                                debug!("Pairing request sent, start waiting for response");
                                Self::wait_for_pairing_response(events, persona_id)
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
    type Error = failure::Error;

    fn into_future(self) -> Self::Future {
        let client_fut = self
            .appctx
            .dapp_service
            .dapp_session(self.appctx.dapp_id.to_owned())
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
                    .map_err(|()| err_msg("Failed to get next event from publisher"))
            });

        Box::new(client_fut)
    }
}
