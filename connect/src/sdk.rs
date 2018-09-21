use std::cell::RefCell;
use std::rc::{Rc, Weak};

use failure::Fail;
use futures::prelude::*;
use futures::{future::{loop_fn, Loop}, select_all, sync::mpsc};
use tokio_core::reactor;

use mercury_home_protocol::*;
use mercury_storage::async::KeyValueStore;
use ::{Call, DAppSession, Relation, client::ProfileGateway};
use ::error::{Error, ErrorKind};



pub type EventSink   = mpsc::Sender<ProfileEvent>;
pub type EventStream = mpsc::Receiver<ProfileEvent>;

pub struct DAppConnect
{
    gateway:        Rc<ProfileGateway>,
    app_id:         ApplicationId,

    session_cache:  Rc<RefCell< Option<Rc<HomeSession>> >>,
    event_sinks:    Rc<RefCell< Vec<EventSink> >>,
    handle:         reactor::Handle,
}


impl DAppConnect
{
    pub fn new(gateway: Rc<ProfileGateway>, app: &ApplicationId, handle: &reactor::Handle) -> Self
        { Self{ gateway, app_id: app.to_owned(), handle: handle.clone(),
                session_cache: Rc::new( RefCell::new(None) ),
                event_sinks: Rc::new( RefCell::new( Vec::new() ) ) } }


    pub fn add_listener(&self, sink: EventSink)
        { self.event_sinks.borrow_mut().push(sink) }


    fn forward_event(mut sinks: Vec<EventSink>, event: ProfileEvent)
        -> Box< Future<Item=Vec<EventSink>, Error=()> >
    {
        let all_send_futs = sinks.drain(..)
            .map( |sink| sink.send( event.clone() ) )
            .collect();

        let fwd_fut = loop_fn( ( Box::new( ::std::iter::empty() ) as Box<Iterator<Item=_>>, all_send_futs),
            |(successful, remaining)|
            {
                select_all(remaining).then( |first_finished_res|
                {
                    let (sent, tail) = match first_finished_res {
                        Err((_err, _idx, tail)) => (Box::new(successful) as Box<Iterator<Item=_>>, tail),
                        Ok((sink, _idx, tail))  => ( Box::new( successful.chain( ::std::iter::once(sink) ) ) as Box<Iterator<Item=_>>, tail ),
                    };

                    if tail.is_empty() { Ok(Loop::Break( (sent,tail) )) }
                    else { Ok(Loop::Continue( (sent,tail) )) }
                } )
            } )
            .map( |(successful, _tail)| successful.collect::<Vec<_>>() );
        Box::new(fwd_fut)
    }


    fn forward_event_res(event_sinks_weak: Weak<RefCell< Vec<EventSink> >>,
                         event_res: Result<ProfileEvent,String>)
        -> Box< Future<Item=(), Error=()> >
    {
        // Get strong Rc from Weak, stop forwarding if Rc is already dropped
        let event_sinks_rc = match event_sinks_weak.upgrade() {
            Some(sinks) => sinks,
            None => return Box::new( Err(()).into_future() ), // NOTE error only to break for_each, otherwise normal
        };

        // Try unwrapping and forwarding event, stop forwarding if received remote error
        match event_res {
            Ok(event) => {
                let sinks = event_sinks_rc.replace( Vec::new() );
                let fwd_fut = Self::forward_event(sinks, event)
                    .map( move |successful_sinks| {
                        let mut listeners = event_sinks_rc.borrow_mut();
                        listeners.extend(successful_sinks);
                    } );
                Box::new(fwd_fut) as Box<Future<Item=(), Error=()>>
            },
            Err(e) => {
                warn!("Remote error listening to profile events, stopping listeners: {}", e);
                Box::new( Err(()).into_future() )
            },
        }
    }


    fn login_and_forward_events(&self) -> Box< Future<Item=Rc<HomeSession>, Error=Error> >
    {
        if let Some(ref session_rc) = *self.session_cache.borrow()
            { return Box::new( Ok( session_rc.clone() ).into_future() ) }

        let login_fut = self.gateway.login()
            .map( {
                let handle = self.handle.clone();
                let session_cache = self.session_cache.clone();
                let listeners = Rc::downgrade(&self.event_sinks);
                move |session| {
                    debug!("Login was successful, start forwarding profile events to listeners");
                    *session_cache.borrow_mut() = Some( session.clone() );
                    handle.spawn( session.events().for_each(
                        move |event| Self::forward_event_res( listeners.clone(), event ) ) );
                    session
                }
            } )
            .map_err( |err| err.context(ErrorKind::Unknown).into() ); // TODO
        Box::new(login_fut)
    }


    // Try fetching RelationProof from existing contacts. If no appropriate contact found,
    // initiate a pairing procedure and return when it's completed, failed or timed out
    // TODO automated pairing should be handled at a different level (near the dApp), not here
    fn get_relation_proof(&self, profile_id: &ProfileId)
        -> Box< Future<Item=RelationProof, Error=Error>>
    {
        let (event_sink, event_stream) = mpsc::channel(CHANNEL_CAPACITY);
        self.add_listener(event_sink);

        let login_fut = self.login_and_forward_events();

        let my_id = self.gateway.signer().profile_id().to_owned();
        let profile_id = profile_id.to_owned();
        let profile_id2 = profile_id.clone();
        let gateway = self.gateway.clone();

        let proof_fut = self.contacts()
            .and_then( move |contacts|
            {
                let first_match = contacts.iter()
                    .map( |relation| relation.proof.to_owned() )
                    .filter( move |proof| {
                        let res = proof.peer_id(&my_id).map( |id| id.to_owned() );
                        res.is_ok() && res.unwrap() == profile_id.clone()
                    })
                    .nth(0);

                match first_match
                {
                    Some(proof) => {
                        debug!("Found proof of contact");
                        Box::new( Ok(proof).into_future() ) as Box<Future<Item=_, Error=_>>
                    },

                    None => {
                        debug!("No proof of contact found, initiate profile pairing");
                        // TODO move this pairing implementation together with the automated event forwarding
                        //      to the ConnectService instead of the SDK here
                        let proof_fut = gateway.pair_request(RelationProof::RELATION_TYPE_ENABLE_CALLS_BETWEEN, &profile_id2, None)
                            .map_err( |err| err.context(ErrorKind::Unknown).into() ) // TODO
                            .inspect( |_| debug!("Pairing request was sent, listen for profile events on my home") )
                            .and_then( |_| login_fut )
                            .and_then( |_session|
                                event_stream.filter_map( move |event|
                                {
                                    debug!("Profile event listener got event");
                                    if let ProfileEvent::PairingResponse(proof) = event {
                                        debug!("Got pairing response, checking peer id: {:?}", proof);
                                        if proof.peer_id( gateway.signer().profile_id() ).is_ok()
                                            { return Some(proof) }
                                    }
                                    return None
                                } )
                                .take(1)
                                .collect()
                                .map_err( |()| {
                                    debug!("Pairing failed");
                                    Error::from(ErrorKind::Unknown) // TODO
                                } )
                            )
                            .and_then( |mut proofs| {
                                debug!("Got {} matching pairing response: {:?}", proofs.len(), proofs.last());
                                proofs.pop().ok_or( {
                                    debug!("Profile event stream ended without proper response");
                                    ErrorKind::Unknown.into()
                                } )
                            } ); // TODO
                        Box::new(proof_fut)
                    }
                }
            } );
        Box::new(proof_fut)
    }
}


// TODO this aims only feature-completeness initially for a HelloWorld dApp,
//      but we also have to include security with authorization and UI-plugins later
impl DAppSession for DAppConnect
{
    fn selected_profile(&self) -> &ProfileId
        { self.gateway.signer().profile_id() }


    fn contacts(&self) -> Box< Future<Item=Vec<Relation>, Error=Error> >
    {
        // TODO properly implement this
        // unimplemented!();
        Box::new( Ok( Vec::new() ).into_future() )
    }


    fn app_storage(&self) -> Box< Future<Item=KeyValueStore<String,String>, Error=Error> >{
        unimplemented!();
    }


    fn checkin(&self) -> Box< Future<Item=HomeStream<Box<IncomingCall>,String>, Error=Error> >
    {
        let checkin_fut = self.login_and_forward_events()
            .and_then( {
                let app = self.app_id.clone();
                move |session| {
                    debug!("Checking in app {:?} to receive incoming calls", app);
                    Ok( session.checkin_app(&app) )
                }
            } );
        Box::new(checkin_fut)
    }


    fn call(&self, profile_id: &ProfileId, init_payload: AppMessageFrame)
        -> Box< Future<Item=Call, Error=Error> >
    {
        debug!("Got call request to {}", profile_id);
        let call_fut = self.get_relation_proof(profile_id)
            .inspect( |_| debug!("Got relation proof, initiate call") )
            .and_then( {
                let gateway = self.gateway.clone();
                let app_id = self.app_id.clone();
                let (to_caller, from_callee) = mpsc::channel(CHANNEL_CAPACITY);
                move |relation| gateway.call(relation.to_owned(), app_id, init_payload, Some(to_caller))
                    .map_err( |e| e.context(ErrorKind::Unknown).into() )
                    .and_then( |to_callee_opt| {
                        debug!("Got response to call");
                        match to_callee_opt {
                            None => Err( Error::from(ErrorKind::Unknown) ), // TODO
                            Some(to_callee) => Ok( Call{ sender: to_callee, receiver: from_callee } )
                        }
                    } )
            } );

        Box::new(call_fut)
    }
}



//pub trait SdkProfileRepository
//{
//    fn create(&self, profile_path: Option<Bip32Path>, own_profile: &OwnProfile) ->
//        Box< Future<Item=(Rc<SdkProfile>, DeviceAuthorization), Error=ErrorToBeSpecified> >;
//
//    fn claim(&self, profile_path: Option<Bip32Path>, auth: Option<DeviceAuthorization>) ->
//        Box< Future<Item=Rc<SdkProfile>, Error=ErrorToBeSpecified> >;
//}
