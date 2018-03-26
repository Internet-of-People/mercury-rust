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
    fn fill_from(&mut self, source: &T);
}


impl<'a> From<&'a [u8]> for ::ProfileId
{
    fn from(src: &'a [u8]) -> Self
        { ::ProfileId( src.to_owned() ) }
}

impl<'a> TryFrom<profile::Reader<'a>> for ::Profile
{
    type Error = capnp::Error;

    fn try_from(src: profile::Reader) -> Result<Self, Self::Error>
    {
        // TODO properly implement this
        let profile_id = ::ProfileId( src.get_id()?.to_owned() );
        let public_key = ::PublicKey( src.get_public_key()?.to_owned() );
        let facets = &[]; // TODO
        Ok( ::Profile::new(&profile_id, &public_key, facets) )
    }
}

impl<'a> FillFrom<::Profile> for profile::Builder<'a>
{
    fn fill_from(&mut self, src: &::Profile)
    {
        self.set_id(&src.id.0);
        self.set_public_key(&src.pub_key.0);
        // TODO set facets
    }
}


//impl<'a> From<::Profile> for profile::Builder<'a>
//{
//    fn from(src: ::Profile) -> Self
//    {
//
//    }
//}
