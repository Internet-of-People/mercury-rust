use super::*;

use std::cell::RefCell;
use std::time::Duration;

use futures::*;
use futures::sync::mpsc::channel;

use tokio_signal::unix::{SIGUSR1};

use ::either::Either;

pub struct Server{
    cfg : ServerConfig,
    appcx : AppContext,
}

impl Server{
    pub fn new(cfg: ServerConfig, appcx : AppContext) -> Self {
        Server{
            cfg : cfg,
            appcx : appcx,
        }
    }
}

impl IntoFuture for Server {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let (tx_call, rx_call) = channel::<Option<AppMsgSink>>(1);
        let (tx_event, rx_event) = channel::<()>(1);

        let rx = rx_call.map(|c| Either::Left(c)).select(rx_event.map(|e| Either::Right(e)));

        let calls_fut = (self.appcx.gateway as Rc<ProfileGateway>).initialize(&ApplicationId("buttondapp".into()), &self.appcx.handle)
        .map_err(|_err| std::io::Error::new(std::io::ErrorKind::Other, "Could not initialize MercuryConnect"))
        .and_then(|mercury_app|{
            mercury_app.checkin()
                .map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, format!("{:?}", err)))
                .and_then(move |call_stream| {
                    call_stream
                        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "call stream failed"))
                        .for_each( move |call_result| {
                            match call_result {
                                Ok(c) => {
                                    let (msgchan_tx, _) = channel(1);
                                    let msgtx = c.answer(Some(msgchan_tx)).to_caller;
                                    Box::new(tx_call.clone().send(msgtx).map(|_| ()).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "failed to send to mpsc"))) as Box<Future<Item=(), Error=std::io::Error>> 
                                },

                                Err(_errmsg) => 
                                    Box::new(Ok(()).into_future()) as Box<Future<Item=(), Error=std::io::Error>>
                            }
                        }) 
                })
        });

        // Handling call and event management
        let calls = RefCell::new(Vec::new());
        let rx_fut = rx.for_each(move |v : Either<Option<AppMsgSink>, ()>| {   
            match v {
                Either::Left(call) => {
                    if let Some(c) = call {
                        calls.borrow_mut().push(c);
                    }
                    
                    Ok(())
                },
                Either::Right(()) => {
                    debug!("notifying connected clients");
                    for c in calls.borrow().iter() {
                        let cc = c.clone();
                        tokio::spawn(cc.send(Ok(AppMessageFrame(b"".to_vec()))).map(|_| ()).map_err(|_|()));
                    }
                    Ok(())
                }
            }
            
            
        }).map_err(|()| std::io::Error::new(std::io::ErrorKind::Other, "mpsc channel failed"));

        // Interval future is generating an event periodcally
        let interval_fut = self.cfg.event_timer.map(|interval| {
            let tx_interval = tx_event.clone();
            Interval::new(std::time::Instant::now() + Duration::from_secs(interval), Duration::from_secs(interval)).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                .for_each(move |_| {
                    info!("interval timer fired, generating event");
                    tx_interval.clone().send(()).map(|_| ()).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                })
        });

        // SIGUSR1 is generating an event
        let tx_sigint1 = tx_event.clone();
        let sigusr1_fut = signal_recv(SIGUSR1).for_each(move |_| {
            info!("received SIGUSR1, generating event");                                    
            tx_sigint1.clone().send(()).map(|_| ()).map_err(|err |std::io::Error::new(std::io::ErrorKind::Other, err))
        });
        
        let mut fut : Box<Future<Item=(), Error=std::io::Error>> = 
            // Box::new(sigint_fut
                //.select(sigusr1_fut).map(|(item, _)| item).map_err(|(err, _)| err)
            Box::new(sigusr1_fut
                .select(rx_fut).map(|(item, _)| item).map_err(|(err, _)| err)
                .select(calls_fut).map(|(item, _)| item).map_err(|(err, _)| err)
        );
        
        if interval_fut.is_some() {
            fut = Box::new(fut
                .select(interval_fut.unwrap()).map(|(item, _)| item).map_err(|(err, _)| err));
        }
        fut
    }
}


