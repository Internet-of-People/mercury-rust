include!( concat!( env!("OUT_DIR"), "/protocol/mercury_capnp.rs" ) );



// TODO add converter functions between business logic and generated Capnproto types
//impl From<::Profile> for Profile
//{
//
//}