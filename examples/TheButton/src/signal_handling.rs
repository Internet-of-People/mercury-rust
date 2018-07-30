use super::*;
use std;

use futures::Poll;

use tokio_signal::unix::Signal;
use tokio_signal::unix::{SIGINT, SIGUSR1, SIGUSR2};

// pub type SignalFuture = futures::Future<Error=std::io::Error, Item=tokio_signal::unix::Signal>;
pub type BoxedSignalFuture = 
    std::boxed::Box< futures::Future<Error=std::io::Error, 
                                     Item=tokio_signal::unix::Signal> 
                     + std::marker::Send>;
pub type SignalStream = futures::FlattenStream<BoxedSignalFuture>;
pub type SignalPoll = Poll<tokio_signal::unix::Signal, std::io::Error>;

pub fn signal_recv(sig : i32)-> SignalStream{
    Signal::new(sig).flatten_stream()
}

trait HandleSignal where Self: Sized{
    type Item;
    type Error;
    fn handle_int(SignalPoll)->Poll<Self::Item, Self::Error>;
    fn handle_usr1(SignalPoll)->Poll<Self::Item, Self::Error>;
    fn handle_usr2(SignalPoll)->Poll<Self::Item, Self::Error>;
}

struct SimpleHandle;
impl HandleSignal for SimpleHandle{
    type Item=i32;
    type Error=std::io::Error;
    fn handle_int(poll: SignalPoll)->Poll<Self::Item, Self::Error>{
        match poll{
            Ok(Async::Ready(Some(_)))=>{
                debug!("SIGINT, closing");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGINT received"));    
            }
            Ok(Async::NotReady)=>{
                return Ok(Async::NotReady);                              
            }
            Ok(Async::Ready(None))=>{
                info!("SIGINT stream closed");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGINT closed"));
            }
            Err(e)=>{
                warn!("SIGINT error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,e.description()));
            }   
        }
    }

    fn handle_usr1(poll: SignalPoll)->Poll<Self::Item, Self::Error>{
        match poll{
            Ok(Async::Ready(Some(_)))=>{
                debug!("SIGUSR1, closing");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR1 received"));    
            }
            Ok(Async::NotReady)=>{
                return Ok(Async::NotReady);               
            }
            Ok(Async::Ready(None))=>{
                info!("SIGUSR1 stream closed");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR1 closed"));
            }
            Err(e)=>{
                warn!("SIGUSR1 error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,e.description()));
            }   
        }
    }

    fn handle_usr2(poll: SignalPoll)->Poll<Self::Item, Self::Error>{
        match poll{
            Ok(Async::Ready(Some(_)))=>{
                debug!("SIGUSR2, closing");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR2 received"));    
            }
            Ok(Async::NotReady)=>{
                return Ok(Async::NotReady);                              
            }
            Ok(Async::Ready(None))=>{
                info!("SIGUSR2 stream closed");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,"SIGUSR2 closed"));
            }
            Err(e)=>{
                warn!("SIGUSR2 error");
                return Err(std::io::Error::new(std::io::ErrorKind::Other,e.description()));
            }   
        }
    }
}

struct SignalHandler<H : HandleSignal>{
    handler: H,
    int : SignalStream,
    usr1: SignalStream,
    usr2: SignalStream,
}

impl<H> SignalHandler<H> where H : HandleSignal{
    pub fn new(handler: H) -> Self{
        Self{
            handler : handler, 
            int : signal_recv(SIGINT),
            usr1: signal_recv(SIGUSR1),
            usr2: signal_recv(SIGUSR2),        
        }
    }
}

impl<H> Future for SignalHandler<H> where H : HandleSignal{
    type Item = u8;
    type Error= std::io::Error;
    fn poll(&mut self)->Poll<Self::Item, Self::Error>{
        self.handler.handle_int(self.int.poll());
        self.handler.handle_usr1(self.usr1.poll());
        self.handler.handle_usr2(self.usr2.poll());   
    }
}