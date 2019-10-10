use std::cell::RefCell;
use std::time::Duration;

use futures::prelude::*;
use futures::sync::mpsc;
use log::*;
use tokio_signal::unix::SIGUSR1;

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
}

impl IntoFuture for Server {
    type Item = ();
    type Error = failure::Error;
    type Future = AsyncResult<Self::Item, Self::Error>;

    fn into_future(self) -> Self::Future {
        // Create dApp session with Mercury Connect and listen for incoming events, automatically accept calls
        let active_calls_rc = self.active_calls.clone();

        let dapp_events_fut = self.appctx.dapp_service.dapp_session(self.appctx.dapp_id.to_owned())
            .inspect( |_| debug!("dApp session was initialized, checking in") )
            .map_err( |err| { error!("Failed to create dApp session: {:?}", err); err } )
            .and_then(|dapp_session| dapp_session.checkin() )
            .inspect( |_call_stream| debug!("Call stream received with successful checkin, listening for calls") )
            .and_then(move |dapp_events|
            {
                dapp_events
                    .map_err( |()| err_msg("Failed to get events") )
                    .for_each( move |event|
                    {
                        match event
                        {
                            DAppEvent::Call(incoming_call) => {
                                let (to_me, from_caller) = mpsc::channel(1);
                                let to_caller_opt = incoming_call.answer(Some(to_me)).to_caller;
                                if let Some(to_caller) = to_caller_opt
                                    { active_calls_rc.borrow_mut().push(
                                        DAppCall{incoming: from_caller, outgoing: to_caller} ); }
                                Ok( debug!("Answered incoming call, saving channel to caller") )
                            },

                            DAppEvent::PairingResponse(response) => Ok( debug!(
                                "Got incoming pairing response. We do not send such requests, ignoring it {:?}", response.proof()) ),
                        }
                    } )
            } );

        // Forward button press events to all interested clients
        let handle = self.appctx.handle.clone();
        let active_calls_rc = self.active_calls.clone();
        let (generate_button_press, got_button_press) = mpsc::channel(CHANNEL_CAPACITY);
        let fwd_pressed_fut = got_button_press
            .for_each(move |()| {
                // TODO use something better here then handle.spawn() for all clients,
                //      we should also detect and remove failing senders
                let calls = active_calls_rc.borrow();
                debug!("Notifying {} connected clients", calls.len());
                for call in calls.iter() {
                    let to_client = call.outgoing.clone();
                    handle.spawn(
                        to_client.send(Ok(AppMessageFrame(vec![42]))).map(|_| ()).map_err(|_| ()),
                    );
                }
                Ok(())
            })
            .map_err(|e| format_err!("Failed to detect next button pressed event: {:?}", e));

        // Receiving a SIGUSR1 signal generates an event
        let button_press_generator = generate_button_press.clone();
        let press_on_sigusr1_fut = signal_recv(SIGUSR1).for_each(move |_| {
            info!("received SIGUSR1, generating event");
            button_press_generator
                .clone()
                .send(())
                .map(|_| ())
                .map_err(|e| format_err!("Failed to fetch next interrupt event: {}", e))
        });

        // Combine all three for_each() tasks to be run in "parallel" on the reactor
        let server_fut = dapp_events_fut
            .select(fwd_pressed_fut)
            .map(|_| ())
            .map_err(|(e, _)| e)
            .select(press_on_sigusr1_fut)
            .map(|_| ())
            .map_err(|(e, _)| e);

        // Combine an optional fourth one if timer option is present
        let server_fut = match self.cfg.event_timer_secs {
            None => Box::new(server_fut) as AsyncResult<_, _>,

            // Repeatedly generate an event with the given interval
            Some(interval_secs) => {
                let press_on_timer_fut =
                    reactor::Interval::new(Duration::from_secs(interval_secs), &self.appctx.handle)
                        .unwrap()
                        .map_err(|e| format_err!("Failed to set button pressed timer: {}", e))
                        .for_each(move |_| {
                            info!("interval timer fired, generating event");
                            generate_button_press.clone().send(()).map(|_| ()).map_err(|e| {
                                format_err!("Failed to send event to subscriber: {}", e)
                            })
                        });

                Box::new(server_fut.select(press_on_timer_fut).map(|_| ()).map_err(|(e, _)| e))
            }
        };

        Box::new(init_publisher(&self).then(|_| server_fut))
    }
}
