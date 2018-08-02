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
/*
    pub fn generate_event(&mut self)-> bool {
        info!("generating event {} {}", self.event_stock, self.sent);

        // iterate connected clients and send them a message

        self.sent += 1;
        match self.cfg.event_count {
            Some(limit) => limit == self.sent,
            _ => false    
        }
    }
*/
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

    //TODO this is unusable right now, and so no multi line socketing will work
    // fn read_some(stream : UnixStream, buf : Vec<u8>){
    //     read(stream, buf)
    //         .map(|(stream, buf, byte)|{
    //             for _ in 0..byte-1{
    //                 Self::generate_event();
    //             }
    //         Self::read_some(stream, buf)
    //     });
    // }

    pub fn stop_event_generation(){
        info!("Stopped event auto-generation");
    }   

/*
    fn handle_sigint(&mut self) -> Option<futures::Poll<i32, std::io::Error>>{
        match self.sig.poll(){
            Ok(Async::Ready(Some(_)))=>{
                debug!("SIGINT, closing");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGINT received")));    
            }
            Ok(Async::NotReady)=>{
                return None;               
            }
            Ok(Async::Ready(None))=>{
                info!("SIGINT stream closed");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGINT stream closed")));
            }
            Err(e)=>{
                warn!("SIGINT error");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,e.description())));
            }   
        }
    }
    fn handle_sigusr1(&mut self) -> Option<futures::Poll<i32, std::io::Error>>{
        match self.usr1.poll(){
            Ok(Async::Ready(Some(_)))=>{
                match self.cfg.event_count{
                    Some(c)=>{
                        if self.sent>=c{
                            return Some(Err(std::io::Error::new(std::io::ErrorKind::Other, "event limit reached"))); 
                        }
                        self.sent += 1;
                    }
                    None=>{();}
                }
                // for call in self.calls{
                //     self.generate_event(&call);
                // }
                return None;    
            }
            Ok(Async::NotReady)=>{
                return None;             
            }
            Ok(Async::Ready(None))=>{
                info!("SIGUSR1 stream closed");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR1 stream closed")));
            }
            Err(e)=>{
                warn!("SIGUSR1 error");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,e.description())));
            }   
        }
    }
    fn handle_sigusr2(&mut self) -> Option<futures::Poll<i32, std::io::Error>>{
        match self.usr2.poll(){
            Ok(Async::Ready(Some(_)))=>{
                self.del = None;
                return None;    
            }
            Ok(Async::NotReady)=>{
                return None;              
            }
            Ok(Async::Ready(None))=>{
                info!("SIGUSR2 stream closed");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR2 stream closed")));
            }
            Err(e)=>{
                warn!("SIGUSR2 error");
                return Some(Err(std::io::Error::new(std::io::ErrorKind::Other,e.description())));
            }   
        }
    }
    */
}

impl IntoFuture for Server {
    type Item = ();
    type Error = std::io::Error;
    type Future = Box<Future<Item=Self::Item, Error=Self::Error>>;

    fn into_future(self) -> Self::Future {
        let (tx_call, rx_call) = channel::<Call>(1);
        let (tx_event, rx_event) = channel::<()>(1);

        let rx = rx_call.map(|c| Either::Left(c)).select(rx_event.map(|e| Either::Right(e)));

        let calls = Rc::new(RefCell::new(Vec::new()));
        rx.for_each(move |v : Either<Call, ()>| {            
            match v {
                Either::Left(call) => 
                    calls.as_ref().borrow_mut().push(call),
                Either::Right(()) => {
                    for c in calls.as_ref().borrow().iter() {
                        // TODO: send message to c
                    }
                }
            }
            
            Ok(())
        });

        // Interval future
        let tx_interval = tx_event.clone();
        let interval_fut = Interval::new(std::time::Instant::now(), Duration::from_secs(3))
            .map_err(move |_| ())
            .for_each(move |_| {
                info!("interval timer fired, generating event");
                tx_interval.clone().send(()).map(|_| ()).map_err(|_| ())
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



        let fut = sigint_fut.select(sigusr1_fut).map(|(item, _)| item).map_err(|(err, _)| err);
        
        Box::new(fut)
    }
}


/*
impl Future for Server{
    type Item = ();
    type Error = std::io::Error;
    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error>{
        match self.handle_sigint(){
            Some(err) => return err,
            None => ()
        }
        match self.handle_sigusr1(){
            Some(err) => return err,
            None => ()
        }
        match self.handle_sigusr2(){
            Some(err) => return err,
            None => ()
        }

        match self.callstream.poll(){
            Ok(Async::Ready(Some(call)))=>{
                debug!("generate_event");
                // self.calls.push(call);
                match call {
                    Ok(c) =>{ 
                        match c.request_details().to_caller{
                            Some(sink)=>{
                                self.calls.push(sink.to_owned());
                            },
                            _=>(),
                        }
                    },//handling of incoming calls is problematic
                    Err(e) =>{ return Ok(Async::NotReady);},
                }
                
                // Ok(Async::NotReady)    
            }
            Ok(Async::NotReady)=>{
                debug!("not ready");
                return Ok(Async::NotReady);                    
            }
            Ok(Async::Ready(None))=>{
                debug!("stream close");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"timer failed"));
            }
            Err(e)=>{
                debug!("error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"TODOe.description()"));
            }
        }

        match self.cfg.event_count{
            Some(c)=>{
                while self.event_stock > 0 {
                    if self.sent >= c {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "event limit reached"));
                    }   
                    self.event_stock-=1;
                    self.sent+=1;
                    for call in self.calls{
                        self.generate_event(&call);
                    }
                }
                ();
            },
            None=>{();}
        }
        match self.uds{
            Some(ref mut incoming)=>{
                let mut gen : u16 = 0;
                match incoming.poll(){
                    Ok(Async::Ready(Some(sock)))=>{
                        let mut s : Vec<u8> = Vec::new();
                        s.resize(10, 1);
                        warn!("whatever");
                        match read(sock, s).poll(){
                            Ok(Async::Ready((_stream, _buf, byte)))=>{
                                gen = byte as u16;
                            }
                            _=>{}
                        }
                    }
                    Ok(Async::NotReady)=>{
                        ()                 
                    }
                    Ok(Async::Ready(None))=>{
                        debug!("stream close");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other,"timer failed"));
                    }
                    Err(e)=>{
                        debug!("error");
                        return Err(std::io::Error::new(std::io::ErrorKind::Other,e.description()));
                    }
                }
                self.event_stock += gen;
            },
            _ => ()
        }

        match self.del{
            Some(ref mut intval)=>{
                loop{
                    match intval.poll(){
                        Ok(Async::Ready(Some(_)))=>{
                            debug!("generate_event");
                            self.event_stock += 1;
                            // Ok(Async::NotReady)    
                        }
                        Ok(Async::NotReady)=>{
                            debug!("not ready");
                            return Ok(Async::NotReady);                    
                        }
                        Ok(Async::Ready(None))=>{
                            debug!("stream close");
                            return Err(std::io::Error::new(std::io::ErrorKind::Other,"timer failed"));
                        }
                        Err(e)=>{
                            debug!("error");
                            return Err(std::io::Error::new(std::io::ErrorKind::Other,e.description()));
                        }
                    }
                }
            }
            None=>{
                Ok(Async::NotReady)
            }
        }
        
    }
}
*/