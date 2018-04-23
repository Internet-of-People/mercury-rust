use capnp;
use capnp::capability::Promise;
use futures;

include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );



pub trait PromiseUtil<T,E>
{
    fn result(result: Result<T,E>) -> Promise<T,E> where T: 'static, E: 'static
        { Promise::from_future( futures::future::result(result) ) }
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
        self.set_id(&src.id.0);
        self.set_public_key(&src.pub_key.0);
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
