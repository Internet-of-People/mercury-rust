use std::cell::RefCell;
use std::time::{Duration, Instant};

use futures::prelude::*;
use futures::select;
use log::*;
use tokio::runtime::current_thread as reactor;
use tokio::sync::mpsc;
use tokio::timer::Interval;
use tokio_net::signal::unix::{signal, SignalKind};

use super::*;
use crate::init::init_publisher;
use crate::options::PublisherConfig;

pub struct Server {
    pub cfg: PublisherConfig,
    pub appctx: AppContext,
    active_calls: Rc<RefCell<Vec<DAppCall>>>,
}

impl Server {
    pub fn new(cfg: PublisherConfig, appctx: AppContext) -> Self {
        Self { cfg, appctx, active_calls: Default::default() }
    }

    pub async fn checkin_and_notify(&self) -> Fallible<()> {
        // Create dApp session with Mercury Connect and listen for incoming events, automatically accept calls
        let dapp_session =
            self.appctx.dapp_service.dapp_session(self.appctx.dapp_id.to_owned()).await?;
        debug!("dApp session was initialized, checking in");

        let mut dapp_events = dapp_session.checkin().await?;
        debug!("Call stream received with successful checkin, listening for calls");

        while let Some(event) = dapp_events.next().await {
            match event {
                DAppEvent::Call(incoming_call) => {
                    let (to_me, from_caller) = mpsc::channel(1);
                    let to_caller_opt = incoming_call.answer(Some(to_me)).to_caller;
                    debug!("Answered incoming call, saving channel to caller");
                    if let Some(to_caller) = to_caller_opt {
                        self.active_calls
                            .borrow_mut()
                            .push(DAppCall { incoming: from_caller, outgoing: to_caller });
                    }
                }

                DAppEvent::PairingResponse(response) => debug!(
                    "Got incoming pairing response. We do not send such requests, ignoring it {:?}",
                    response.proof()
                ),
            }
        }

        Ok(())
    }

    pub async fn publish_button_presses(
        &self,
        mut press_events: impl Stream<Item = ()> + Unpin,
    ) -> Fallible<()> {
        // Forward button press events to all interested clients
        while let Some(()) = press_events.next().await {
            let calls = self.active_calls.borrow();
            debug!("Notifying {} connected clients", calls.len());
            for call in calls.iter() {
                // TODO use something better here then spawn() for all clients,
                //      we should also detect and remove failing senders
                let mut out_sink = call.outgoing.to_owned();
                reactor::spawn(async move {
                    let res = out_sink.send(Ok(AppMessageFrame(vec![42]))).await;
                    match res {
                        Ok(()) => {}
                        Err(e) => warn!("Failed to send: {}", e),
                    }
                });
            }
        }
        Ok(())
    }

    pub async fn timed_button_press(
        &self,
        mut generate_button_press: impl Sink<()> + Unpin,
    ) -> Fallible<()> {
        let timer_secs = match self.cfg.event_timer_secs {
            None => return Ok(()),
            // Repeatedly generate an event with the given interval
            Some(interval_secs) => interval_secs,
        };

        let mut tick_stream = Interval::new(Instant::now(), Duration::from_secs(timer_secs));
        while let Some(tick) = tick_stream.next().await {
            info!("interval timer fired, generating event");
            generate_button_press.send(()).await.map_err(|e| err_msg("Implementation error"))?;
        }

        Ok(())
    }

    pub async fn run(&self) -> Fallible<()> {
        init_publisher(self).await?;

        let (generate_button_press, got_button_press) = mpsc::channel(CHANNEL_CAPACITY);

        // Combine all tasks to be run in "parallel" on the reactor
        select! {
            _ = self.checkin_and_notify().boxed_local().fuse() => {},
            _ = self.publish_button_presses(got_button_press).boxed_local().fuse() => {},
            _ = self.timed_button_press(generate_button_press).boxed_local().fuse() => {},
            // TODO handle signals to generate events
        }

        Ok(())
    }

    //    pub async fn todo() -> Fallible<()> {
    //        // Receiving a SIGUSR1 signal generates an event
    //        let press_on_sigusr1_fut = signal_recv(SIGUSR1).for_each(move |_| {
    //            info!("received SIGUSR1, generating event");
    //            button_press_generator
    //                .clone()
    //                .send(())
    //                .map(|_| ())
    //                .map_err(|e| format_err!("Failed to fetch next interrupt event: {}", e))
    //        });
    //    }
}
