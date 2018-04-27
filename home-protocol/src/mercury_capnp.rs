use capnp;
use capnp::capability::Promise;
use futures::prelude::*;
use futures::{future, Sink, sync::mpsc};
use tokio_core::reactor;

use ::{AppMessageFrame, AppMsgSink};

include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );



pub trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( future::result(result) ) }
}

impl<T,E> PromiseUtil<T,E> for Promise<T,E> {}



// NOTE this is identical to the currently experimental std::convert::TryFrom.
//      Hopefully this will not be needed soon when it stabilizes.
pub trait TryFrom<T> : Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
}

pub trait FillFrom<T>
{
    fn fill_from(self, source: &T);
}


impl<'a> From<&'a [u8]> for ::ProfileId
{
    fn from(src: &'a [u8]) -> Self
        { ::ProfileId( src.to_owned() ) }
}

impl<'a> From<&'a ::ProfileId> for &'a [u8]
{
    fn from(src: &'a ::ProfileId) -> Self
        { &src.0 }
}


impl<'a> From<&'a [u8]> for ::AppMessageFrame
{
    fn from(src: &'a [u8]) -> Self
        { ::AppMessageFrame( src.to_owned() ) }
}

impl<'a> From<&'a ::AppMessageFrame> for &'a [u8]
{
    fn from(src: &'a ::AppMessageFrame) -> Self
        { &src.0 }
}


impl<'a> From<&'a str> for ::ApplicationId
{
    fn from(src: &'a str) -> Self
        { ::ApplicationId( src.to_owned() ) }
}

impl<'a> From<&'a ::ApplicationId> for &'a str
{
    fn from(src: &'a ::ApplicationId) -> Self
        { &src.0 }
}


impl<'a> TryFrom<profile::Reader<'a>> for ::Profile
{
    type Error = capnp::Error;

    fn try_from(src: profile::Reader) -> Result<Self, Self::Error>
    {
        let profile_id = ::ProfileId( src.get_id()?.to_owned() );
        let public_key = ::PublicKey( src.get_public_key()?.to_owned() );
        let facets = &[]; // TODO
        Ok( ::Profile::new(&profile_id, &public_key, facets) )
    }
}

impl<'a> FillFrom<::Profile> for profile::Builder<'a>
{
    fn fill_from(mut self, src: &::Profile)
    {
        self.set_id( (&src.id).into() );
        self.set_public_key( &src.pub_key.0 ); // TODO would be nicer with pubkey.into() implementing From<PublicKey>
        // TODO set facets
    }
}


impl<'a> TryFrom<own_profile::Reader<'a>> for ::OwnProfile
{
    type Error = capnp::Error;

    fn try_from(src: own_profile::Reader) -> Result<Self, Self::Error>
    {
        let profile = ::Profile::try_from( src.get_profile()? )?;
        let private_data = src.get_private_data()?;
        Ok( ::OwnProfile::new(&profile, &private_data) )
    }
}

impl<'a> FillFrom<::OwnProfile> for own_profile::Builder<'a>
{
    fn fill_from(mut self, src: &::OwnProfile)
    {
        self.set_private_data(&src.priv_data);
        self.init_profile().fill_from(&src.profile);
    }
}


impl<'a> TryFrom<home_invitation::Reader<'a>> for ::HomeInvitation
{
    type Error = capnp::Error;

    fn try_from(src: home_invitation::Reader) -> Result<Self, Self::Error>
    {
        // TODO
        Ok( ::HomeInvitation::new( &::ProfileId("TODO".as_bytes().to_owned()),
                                   &"TODO", &::Signature("TODO".as_bytes().to_owned() ) ) )
    }
}

impl<'a> FillFrom<::HomeInvitation> for home_invitation::Builder<'a>
{
    fn fill_from(mut self, src: &::HomeInvitation)
    {
        // TODO
    }
}


impl<'a> TryFrom<relation_half_proof::Reader<'a>> for ::RelationHalfProof
{
    type Error = capnp::Error;

    fn try_from(src: relation_half_proof::Reader) -> Result<Self, Self::Error>
    {
        // TODO
        Ok( ::RelationHalfProof::new() )
    }
}

impl<'a> FillFrom<::RelationHalfProof> for relation_half_proof::Builder<'a>
{
    fn fill_from(mut self, src: &::RelationHalfProof)
    {
        // TODO
    }
}


impl<'a> TryFrom<relation_proof::Reader<'a>> for ::RelationProof
{
    type Error = capnp::Error;

    fn try_from(src: relation_proof::Reader) -> Result<Self, Self::Error>
    {
        // TODO
        Err( capnp::Error::failed(String::from("unimplemented try_from")) )
    }
}

impl<'a> FillFrom<::RelationProof> for relation_proof::Builder<'a>
{
    fn fill_from(mut self, src: &::RelationProof)
    {
        // TODO
    }
}



impl<'a> TryFrom<profile_event::Reader<'a>> for ::ProfileEvent
{
    type Error = capnp::Error;

    fn try_from(src: profile_event::Reader) -> Result<Self, Self::Error>
    {
        // TODO
        Ok( ::ProfileEvent::Unknown( Vec::new() ) )
    }
}

impl<'a> FillFrom<::ProfileEvent> for profile_event::Builder<'a>
{
    fn fill_from(mut self, src: &::ProfileEvent)
    {
        // TODO
    }
}



impl<'a> TryFrom<call::Reader<'a>> for ::Call
{
    type Error = capnp::Error;

    // NOTE this cannot fill in streams here without outer context (e.g. reactor::Handle)
    fn try_from(src: call::Reader) -> Result<Self, Self::Error>
    {
        let caller_id = src.get_caller_id()?.into();
        let init_payload = src.get_init_payload()?.into();

        Ok( ::Call{ caller_id: caller_id, init_payload: init_payload,
                    incoming: None, outgoing: None, } )
    }
}

impl<'a> FillFrom<::Call> for call::Builder<'a>
{
    fn fill_from(mut self, src: &::Call)
    {
        self.set_caller_id( (&src.caller_id).into() );
        self.set_init_payload( (&src.init_payload).into() );
        // TODO set up channel to caller: is it possible here without external context?
        // self.set_to_caller( TODO );
    }
}



// TODO consider using a single generic imlementation for all kinds of Dispatchers
pub struct AppMessageDispatcherCapnProto
{
    sender: AppMsgSink,
}

impl AppMessageDispatcherCapnProto
{
    pub fn new(sender: AppMsgSink) -> Self
        { Self{ sender: sender } }
}

impl app_message_listener::Server for AppMessageDispatcherCapnProto
{
    fn receive(&mut self, params: app_message_listener::ReceiveParams,
               _results: app_message_listener::ReceiveResults)
        -> Promise<(), ::capnp::Error>
    {
        let message = pry!( pry!( params.get() ).get_message() );
        let recv_fut = self.sender.clone().send( Ok( message.into() ) )
            .map(  |_sink| () )
            .map_err( |e| ::capnp::Error::failed( format!("Failed to send event: {:?}",e ) ) );
        Promise::from_future(recv_fut)
    }


    fn error(&mut self, params: app_message_listener::ErrorParams,
             _results: app_message_listener::ErrorResults)
        -> Promise<(), ::capnp::Error>
    {
        let error = pry!( pry!( params.get() ).get_error() ).into();
        let recv_fut = self.sender.clone().send( Err(error) )
            .map(  |_sink| () )
            .map_err( |e| ::capnp::Error::failed( format!("Failed to send event: {:?}",e ) ) );
        Promise::from_future(recv_fut)
    }
}



pub fn fwd_appmsg(to_callee: app_message_listener::Client, handle: reactor::Handle) -> AppMsgSink
{
    let (send, recv) = mpsc::channel::<Result<AppMessageFrame, String>>(1);

    handle.spawn(
        recv.for_each( move |message|
        {
            let capnp_fut = match message
            {
                Ok(msg) => {
                    let mut request = to_callee.receive_request();
                    request.get().set_message(&msg.0);
                    let fut = request.send().promise
                        .map(  |_resp| () );
                    Box::new(fut) as Box< Future<Item=(), Error=::capnp::Error> >
                },
                Err(err) => {
                    let mut request = to_callee.error_request();
                    request.get().set_error(&err);
                    let fut = request.send().promise
                        .map(  |_resp| () );
                    Box::new(fut)
                }
            };
            capnp_fut.map_err(  |_e| () ) // TODO what to do here with the network capnp error?
        } )
    );

    send
}
