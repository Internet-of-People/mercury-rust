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

    pub async fn pair_and_listen(&self) -> Fallible<()> {
        let dapp_session =
            self.appctx.dapp_service.dapp_session(self.appctx.dapp_id.to_owned()).await?;

        let contact = self.get_or_create_contact(dapp_session.as_ref()).await?;
        info!("Contact is available, start calling");

        let mut call = contact.call(AppMessageFrame(vec![])).await?;
        info!("call accepted, waiting for incoming messages");

        while let Some(msg) = call.incoming.next().await {
            match msg {
                Ok(frame) => info!("Client received server message {:?}", frame),
                Err(e) => warn!("Client got server error {:?}", e),
            }
        }

        Ok(())
    }

    async fn get_or_create_contact(
        &self,
        dapp_session: &dyn DAppSession,
    ) -> Fallible<Box<dyn Relation>> {
        let peer_id = &self.cfg.server_id;
        let relation_opt = dapp_session.relation(peer_id).await?;

        if let Some(relation) = relation_opt {
            return Ok(relation);
        }

        debug!("No signed relation to server is available, initiate pairing");
        let mut dapp_events = dapp_session.checkin().await?;

        dapp_session.initiate_relation(peer_id).await?;
        debug!("Pairing request sent, start waiting for response");

        while let Some(event) = dapp_events.next().await {
            debug!("TheButton got event");

            if let DAppEvent::PairingResponse(relation) = event {
                debug!("Got pairing response, checking peer id: {:?}", relation.proof());
                let got_peer_id_res = relation.proof().peer_id(dapp_session.profile_id());
                match got_peer_id_res {
                    Ok(id) if id == peer_id => return Ok(relation),
                    Ok(id) => info!("Ignored unexpected pairing response: {}", id),
                    Err(e) => warn!(
                        "Received unexpected relation of another persona: {:?}",
                        relation.proof()
                    ),
                }
            }
        }

        bail!("Failed to receive pairing response");
    }
}
