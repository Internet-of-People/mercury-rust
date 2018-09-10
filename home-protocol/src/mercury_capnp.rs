use capnp;
use capnp::capability::Promise;
use futures::prelude::*;
use futures::{future, Sink, sync::mpsc};
use tokio_core::reactor;

use ::{AppMessageFrame, AppMsgSink, TryFrom};

include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );



pub trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( future::result(result) ) }
}

impl<T,E> PromiseUtil<T,E> for Promise<T,E> {}



pub trait FillFrom<T>
{
    fn fill_from(self, source: &T);
}

impl<'a> From<&'a ::PublicKey> for &'a [u8] {
    fn from(public_key: &'a ::PublicKey) -> Self {
        public_key.0.as_ref()
    }
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

        let facet_res = match src.get_facet().which() {
            Ok(profile::facet::Which::Persona(r)) => {
                if let Some(proof_reader) = r.get_homes()?.iter().next() {  // only 0 or 1 home is supported in the current impl
                    let home_relation = ::RelationProof::try_from(proof_reader)?;
                    Ok(::ProfileFacet::Persona(::PersonaFacet{homes: vec![home_relation], data: vec![]}))
                } else {
                    Ok(::ProfileFacet::Persona(::PersonaFacet{homes: vec![], data: vec![]}))
                }
            },
            // TODO finish this implementation to be able to send HomeProfiles, too
            // Ok(profile::facet::Which::Home(r)) => {
            //     let addrs = r.get_addresses()?.iter().map(|addr| ::Multiaddr::from(addr?));
            //     Ok(::ProfileFacet::Home(::HomeFacet{addrs, data: vec![]}))
            // },
            _ => {
                Err("Unimplemented")
            }
        };

        match facet_res {
            Ok(facet) => Ok(::Profile::new(&profile_id, &public_key, &facet) ),
            Err(e) => Err(::capnp::Error::failed(e.to_owned())),
        }
    }
}

impl<'a> FillFrom<::Profile> for profile::Builder<'a>
{
    fn fill_from(mut self, src: &::Profile)
    {
        self.set_id( (&src.id).into() );
        self.set_public_key( (&src.public_key).into() );
        match src.facet {
            ::ProfileFacet::Persona(ref facet) => {
                let persona_builder = self.init_facet().init_persona();
                let mut homes = persona_builder.init_homes(facet.homes.len() as u32);
                for (i, home) in facet.homes.iter().enumerate() {
                    homes.reborrow().get(i as u32).fill_from(&home);
                }
            }
            _ => {
                panic!("Unimplemented");  // TODO implement home and application facets
            }
        }
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

    fn try_from(_src: home_invitation::Reader) -> Result<Self, Self::Error>
    {
        // TODO
        Ok( ::HomeInvitation::new( &::ProfileId("TODO".as_bytes().to_owned()),
                                   &"TODO", &::Signature("TODO".as_bytes().to_owned() ) ) )
    }
}

impl<'a> FillFrom<::HomeInvitation> for home_invitation::Builder<'a>
{
    // fn fill_from(mut self, src: &::HomeInvitation)
    fn fill_from(self, _src: &::HomeInvitation)
    {
        // TODO
    }
}


impl<'a> TryFrom<relation_half_proof::Reader<'a>> for ::RelationHalfProof
{
    type Error = capnp::Error;

    fn try_from(src: relation_half_proof::Reader) -> Result<Self, Self::Error>
    {
        Ok(::RelationHalfProof {
            relation_type: String::from(src.get_relation_type()?),
            signer_id: ::ProfileId(src.get_signer_id()?.to_owned()),
            peer_id: ::ProfileId(src.get_peer_id()?.to_owned()),
            signature: ::Signature(src.get_signature()?.to_owned()),
        })
    }
}

impl<'a> FillFrom<::RelationHalfProof> for relation_half_proof::Builder<'a>
{
    fn fill_from(mut self, src: &::RelationHalfProof)
    {
        self.set_relation_type(&src.relation_type);
        self.set_signer_id(&src.signer_id.0);
        self.set_peer_id(&src.peer_id.0);
        self.set_signature(&src.signature.0);
    }
}


impl<'a> TryFrom<relation_proof::Reader<'a>> for ::RelationProof
{
    type Error = capnp::Error;

    fn try_from(src: relation_proof::Reader) -> Result<Self, Self::Error>
    {
        Ok(::RelationProof {
            relation_type: String::from(src.get_relation_type()?),
            a_id: ::ProfileId(src.get_a_id()?.to_owned()),
            a_signature: ::Signature(src.get_a_signature()?.to_owned()),
            b_id: ::ProfileId(src.get_b_id()?.to_owned()),
            b_signature: ::Signature(src.get_b_signature()?.to_owned()),
        })
    }
}

impl<'a> FillFrom<::RelationProof> for relation_proof::Builder<'a>
{
    fn fill_from(mut self, src: &::RelationProof)
    {
        self.set_relation_type(&src.relation_type);
        self.set_a_id(&src.a_id.0);
        self.set_a_signature(&src.a_signature.0);
        self.set_b_id(&src.b_id.0);
        self.set_b_signature(&src.b_signature.0);
    }
}



impl<'a> TryFrom<profile_event::Reader<'a>> for ::ProfileEvent
{
    type Error = capnp::Error;

    fn try_from(src: profile_event::Reader) -> Result<Self, Self::Error>
    {
        match src.which()? {
            profile_event::Which::Unknown(data) => Ok(::ProfileEvent::Unknown(Vec::from(data?))),
            profile_event::Which::PairingRequest(half_proof) => Ok(::ProfileEvent::PairingRequest(::RelationHalfProof::try_from(half_proof?)?)),
            profile_event::Which::PairingResponse(proof) => Ok(::ProfileEvent::PairingResponse(::RelationProof::try_from(proof?)?)),
        }
    }
}

impl<'a> FillFrom<::ProfileEvent> for profile_event::Builder<'a>
{
    fn fill_from(self, src: &::ProfileEvent)
    {
        match src {
            ::ProfileEvent::PairingRequest(half_proof) => {
                let mut builder = self.init_pairing_request();
                builder.reborrow().fill_from(half_proof);
            },
            ::ProfileEvent::PairingResponse(proof) => {
                let mut builder = self.init_pairing_response();
                builder.reborrow().fill_from(proof);
            },
            ::ProfileEvent::Unknown(data) => {
                let _builder = self.init_unknown(data.len() as u32);
                // TODO fill with data
                // builder.reborrow().fill_with(data);
            },
        };
    }
}



impl<'a> TryFrom<call_request::Reader<'a>> for ::CallRequestDetails
{
    type Error = capnp::Error;

    // NOTE this cannot fill in streams here without outer context (e.g. reactor::Handle)
    fn try_from(src: call_request::Reader) -> Result<Self, Self::Error>
    {
        let relation = ::RelationProof::try_from( src.get_relation()? )?;
        let init_payload = src.get_init_payload()?.into();

        Ok( ::CallRequestDetails { relation: relation, init_payload: init_payload, to_caller: None } )
    }
}

impl<'a> FillFrom<::CallRequestDetails> for call_request::Builder<'a>
{
    fn fill_from(mut self, src: &::CallRequestDetails)
    {
        self.set_init_payload( (&src.init_payload).into() );
        self.init_relation().fill_from(&src.relation);
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

#[cfg(test)]
mod tests
{
    use ::*;
    use mercury_capnp::FillFrom;
    use capnp::serialize;

    #[test]
    fn relation_half_proof_encoding() {
        let relation_half_proof = ::RelationHalfProof {
            relation_type: String::from("friend"),
            signer_id: ::ProfileId(Vec::from("me")),
            peer_id: ProfileId(Vec::from("you")),
            signature: Signature(Vec::from("i signed")),
        };
        let mut message = capnp::message::Builder::new_default();
        {
            let builder = message.init_root::<mercury_capnp::relation_half_proof::Builder>();
            builder.fill_from(&relation_half_proof);
        }
        let mut buffer = vec![];
        serialize::write_message(&mut buffer, &message).unwrap();
        // -- 8< --
        let message_reader = serialize::read_message(&mut &buffer[..], ::capnp::message::ReaderOptions::new()).unwrap();
        let obj_reader = message_reader.get_root::<mercury_capnp::relation_half_proof::Reader>().unwrap();
        let recoded = RelationHalfProof::try_from(obj_reader).unwrap();
        assert_eq!(recoded, relation_half_proof);
    }
}
