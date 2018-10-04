use std::cell::RefCell;
use std::time::Duration;

use either::Either;
use futures::prelude::*;
use futures::sync::mpsc;
use tokio_signal::unix::SIGUSR1;

use super::*;
use ::init_hack::init_server;



pub struct Server{
    pub cfg : ServerConfig,
    pub appctx: AppContext,
}

impl Server{
    pub fn new(cfg: ServerConfig, appctx: AppContext) -> Self
        { Self{cfg, appctx} }
}



impl IntoFuture for Server {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let (tx_call, rx_call) = mpsc::channel::<Option<AppMsgSink>>(1);
        let (tx_event, rx_event) = mpsc::channel::<()>(1);

        let rx = rx_call.map(|c| Either::Left(c)).select(rx_event.map(|e| Either::Right(e)));

        let calls_fut = self.appctx.service.dapp_session(&ApplicationId("buttondapp".into()), None )
            .inspect( |_app| debug!("dApp session was initialized, checking in") )
            .map_err( |err| { debug!("Failed to create dApp session: {:?}", err); std::io::Error::new(std::io::ErrorKind::Other, "Could not initialize MercuryConnect") })
            .and_then(|mercury_app| mercury_app.checkin()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err))) )
            .inspect( |_call_stream| debug!("Call stream received with successful checkin, listening for calls") )
            .and_then(move |call_stream| { call_stream
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "Failed to receive call stream"))
                .for_each( move |call_result| {
                    match call_result {
                        Ok(DAppEvent::Call(c)) => {
                            let (msgchan_tx, _) = mpsc::channel(1);
                            let msgtx = c.answer(Some(msgchan_tx)).to_caller;
                            let fut = tx_call.clone().send(msgtx).map(|_| ())
                                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "failed to send to mpsc"));
                            Box::new(fut) as Box<Future<Item=_, Error=_>>
                        },

                        Ok(DAppEvent::PairingResponse(response)) => {
                            debug!("Got incoming pairing response, ignoring it {:?}", response);
                            Box::new(Ok(()).into_future())
                        }
                        Err(err) => {
                            debug!("Received error for events, closing event stream: {}", err);
                            Box::new(Err(std::io::Error::new(std::io::ErrorKind::Other, "Call stream got error")).into_future())
                        }
                    }
                })
            });

        let calls_fut = init_server(&self)
            .then( |_| calls_fut );

        // Handling call and event management
        let calls = RefCell::new(Vec::new());
        let handle = self.appctx.handle.clone();
        let rx_fut = rx.for_each(move |v : Either<Option<AppMsgSink>, ()>| {   
            match v {
                Either::Left(call) => {
                    if let Some(c) = call {
                        calls.borrow_mut().push(c);
                    }
                },
                Either::Right(()) => {
                    debug!("notifying connected clients");
                    for c in calls.borrow().iter() {
                        let cc = c.clone();
                        handle.spawn(cc.send(Ok(AppMessageFrame(b"".to_vec())))
                            .map(|_| ()).map_err(|_|()));
                    }
                }
            }
            Ok(())
        }).map_err(|()| std::io::Error::new(std::io::ErrorKind::Other, "mpsc channel failed"));

        // SIGUSR1 is generating an event
        let tx_interval = tx_event.clone();
        let sigusr1_fut = signal_recv(SIGUSR1).for_each(move |_| {
            info!("received SIGUSR1, generating event");                                    
            tx_event.clone().send(()).map(|_| ())
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
        });

        let server_fut = rx_fut
            .select(calls_fut).map(|_| ()).map_err(|(e,_)| e)
            .select(sigusr1_fut).map(|_| ()).map_err(|(e,_)| e);

        match self.cfg.event_timer {
            None => Box::new(server_fut), // as Box<Future<Item=_,Error=_>>,
            Some(interval) => {
                // Interval future is generating an event periodcally
                let handle = self.appctx.handle.clone();
                let timer_fut = reactor::Interval::new( Duration::from_secs(interval) , &handle ).unwrap()
                    .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                    .for_each(move |_| {
                        info!("interval timer fired, generating event");
                        tx_interval.clone().send(()).map(|_| ()).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                    });

                Box::new( server_fut.select(timer_fut)
                    .map(|_| ()).map_err(|(e,_)| e) )
            },
        }
    }
}
