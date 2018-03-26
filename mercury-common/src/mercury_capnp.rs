use capnp;

include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );



// NOTE this is identical to the currently experimental std::convert::TryFrom.
//      Hopefully this will not be needed soon when it stabilizes.
pub trait TryFrom<T> : Sized {
    type Error;
    fn try_from(value: T) -> Result<Self, Self::Error>;
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


//impl<'a> From<::Profile> for profile::Builder<'a>
//{
//    fn from(src: ::Profile) -> Self
//    {
//
//    }
//}
