use super::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;

use futures::*;
use futures::sync::mpsc::channel;

use mercury_connect::sdk::Call;
use mercury_home_protocol::AppMessageFrame;

use ::either::Either;

pub struct Server{
    cfg : ServerConfig,
}

impl Server{
    pub fn default()->Self{
        Self{
            cfg : ServerConfig::new(),
        }   
    }

    pub fn new(cfg: ServerConfig) -> Self {
        /*
        let mut intval = None;
        let mut uds = None;
        if let Some(delay) = cfg.event_timer{
            intval = Some(Interval::new(std::time::Instant::now(), std::time::Duration::new(delay,0)));
        };

        if let Some(file) = cfg.event_file.clone(){
            let mut path = String::from("\0");
            path.push_str(&file);
            path.push_str(".sock");
            let sock_path = std::path::PathBuf::from(path);
            let server = UnixListener::bind(&sock_path);
            match server {
                Ok(serv) => {
                    uds = Some(serv.incoming());
                },
                Err(_e) => {/*TODO*/}
            }
        };
        let mut reactor = tokio_core::reactor::Core::new().unwrap();
        let callstream = reactor.run(connect.checkin()).unwrap();
        */
        Server{
            cfg : cfg,
        }
    }
    // fn handle_event_file(file_name: String)->u32{
    //     let mut path = String::from("\0");
    //     path.push_str(&file_name);
    //     path.push_str(".sock");
    //     let sock_path = std::path::PathBuf::from(path);
    //     let server = t!(UnixListener::bind(&sock_path));
    //     let uds_incoming = server.incoming()
    //         .for_each(move | sock| {
    //             let mut s : Vec<u8> = Vec::new();
    //             s.resize(10, 1);
    //             read(sock, s)
    //                 .map(|(stream, buf, byte)|{
    //                     Self::generate_x(byte as u32);
    //                     //Self::read_some(stream, buf);
    //                 })
    //                 .then(move |_|future::ok(()))
    //         }).then(|_| Ok(()));
    //     tokio::run(uds_incoming);
    // }
}

impl IntoFuture for Server {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let (tx_call, rx_call) = channel::<Call>(1);
        let (tx_event, rx_event) = channel::<()>(1);

        let rx = rx_call.map(|c| Either::Left(c)).select(rx_event.map(|e| Either::Right(e)));

        // let calls = Rc::new(RefCell::new(Vec::new()));
        let calls = RefCell::new(Vec::new());
        rx.for_each(move |v : Either<Call, ()>| {            
            match v {
                Either::Left(call) => 
                    calls./*as_ref().*/borrow_mut().push(call),
                Either::Right(()) => {
                    for c in calls/*.as_ref()*/.borrow().iter() {
                        // TODO: send message to c
                    }
                }
            }
            
            Ok(())
        });

        // Interval future
        let interval_fut = self.cfg.event_timer.map(|interval| {
            let tx_interval = tx_event.clone();
            Interval::new(std::time::Instant::now(), Duration::from_secs(interval)).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                .for_each(move |_| {
                    info!("interval timer fired, generating event");
                    // tx_interval.clone().send(()).map(|_| ()).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                    tx_interval.clone().send(()).map(|_| ()).map_err(|err| std::io::Error::new(std::io::ErrorKind::Other, err))
                })
        });



        // SIGINT is terminating the server
        let sigint_fut = signal_recv(SIGINT).into_future()
            .map(|_| {
                info!("received SIGINT, terminating server");
                ()
            })
            .map_err(|(err, _)| err);

        // SIGUSR1 is generating an event
        let tx_sigint1 = tx_event.clone();
        let sigusr1_fut = signal_recv(SIGUSR1).for_each(move |_| {
            info!("received SIGUSR1, generating event");            
                        
            tx_sigint1.clone().send(()).map(|_| ()).map_err(|err |std::io::Error::new(std::io::ErrorKind::Other, err))
        });

        // SIGUSR1 is generating an event
        let sigusr2_fut = signal_recv(SIGUSR2).for_each(move |_| {
            info!("received SIGUSR2, toggling automatic event generation");            
            Ok(())
        });
        
        let mut fut : Box<Future<Item=(), Error=std::io::Error>>= Box::new(sigint_fut
                    .select(sigusr1_fut).map(|(item, _)| item).map_err(|(err, _)| err)
                    .select(sigusr2_fut).map(|(item, _)| item).map_err(|(err, _)| err));
        
        if interval_fut.is_some() {
            fut = Box::new(fut
                .select(interval_fut.unwrap()).map(|(item, _)| item).map_err(|(err, _)| err));
        }
        fut
    }
}


