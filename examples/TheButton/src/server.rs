use super::*;
use std;
use std::error::Error;

use futures::Stream;

use mercury_connect::sdk::Call;
use mercury_home_protocol::AppMessageFrame;

use futures::Sink;

pub struct Server{
    del: Option<Interval>,
    event_stock : u16,
    sent : u32,
    uds : Option<Incoming>,
    pub cfg : ServerConfig,
    sig : SigStream,
    usr1 : SigStream,
    usr2 : SigStream,
    connect: DAppConnect,
    calls : Vec<Box<Call>>
}

impl Server{
    pub fn default(connect: DAppConnect)->Self{
        Self{
            sent : 0,
            del : None,
            uds : None,
            event_stock : 0,
            cfg : ServerConfig::new(),
            sig : signal_recv(SIGINT),
            usr1 : signal_recv(SIGUSR1),
            usr2 : signal_recv(SIGUSR2),
            connect: connect,
            calls: Vec::new(),
        }
    }

    pub fn new(cfg: ServerConfig, connect: DAppConnect)->
    Self{
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

        Server{
            sent : 0,
            cfg : cfg,
            uds : uds,
            del : intval,
            event_stock : 0,
            sig : signal_recv(SIGINT),
            usr1 : signal_recv(SIGUSR1),
            usr2 : signal_recv(SIGUSR2),
            connect: connect,
            calls: Vec::new(),
        }
    }

    pub fn generate_event(&mut self, call: &AppMsgSink)->i32{
        info!("Generating event {} {}", self.event_stock, self.sent);
        call.send(Ok(AppMessageFrame("event".into())));
        42
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
                for call in self.calls{
                    self.generate_event(call.sender());
                }
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
}


impl Future for Server{
    type Item = i32;
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

        match self.connect.checkin().poll(){
                        Ok(Async::Ready(Some(call)))=>{
                            debug!("generate_event");
                            self.calls.app(call);
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

        match self.cfg.event_count{
            Some(c)=>{
                while self.event_stock > 0 {
                    if self.sent >= c {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, "event limit reached"));
                    }   
                    self.event_stock-=1;
                    self.sent+=1;
                    for call in self.calls{
                        self.generate_event(call.sender());
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
