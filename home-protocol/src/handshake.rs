use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use std::error::Error;
use std::mem;
use std::rc::Rc;

use bincode::{deserialize, serialize};
use futures::{future, Future};
use tokio_core::net::TcpStream;
use tokio_io::io;

use ::*;


#[derive(Deserialize, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
struct AuthenticationInfo
{
    profile_id: ProfileId,
    public_key: PublicKey,
}


pub fn temp_handshake_until_tls_is_implemented(conn: TcpStream, signer: Rc<Signer>)
    -> Box< Future<Item=(TcpStream, PeerContext), Error=ErrorToBeSpecified> >
{
    let auth_info = AuthenticationInfo{
        profile_id: signer.profile_id().to_owned(), public_key: signer.public_key().to_owned() };

    let out_bytes = match serialize(&auth_info) {
        Ok(data) => data,
        Err(e) => return Box::new( future::err( ErrorToBeSpecified::TODO( e.description().to_owned() ) ) ),
    };
    let bufsize = out_bytes.len() as u32;

    let mut size_out_bytes = BytesMut::with_capacity( mem::size_of_val(&bufsize) );
    size_out_bytes.put_u32_le(bufsize);

    let handshake_fut = io::write_all(conn, size_out_bytes)
        .and_then( move |(conn, _buf)| { io::write_all(conn, out_bytes) } )
        .and_then( move |(conn, _buf)|
        {
            let mut size_bytes = BytesMut::with_capacity( mem::size_of_val(&bufsize) );
            io::read_exact(conn, size_bytes)
        } )
        .and_then( |(conn, buf)|
        {
            let size_in_bytes = Bytes::from(buf).into_buf().get_u32_le();
            let in_bytes = BytesMut::with_capacity(size_in_bytes as usize);
            io::read_exact(conn, in_bytes)
        } )
        .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
        .and_then( |(conn, buf)|
        {
            let peer_auth: AuthenticationInfo = deserialize(&buf)
                .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;

//            let peer_auth: AuthenticationInfo = match deserialize(&buf)
//            {
//                Ok(auth) => auth,
//                Err(e) => return Err(ErrorToBeSpecified::TODO( e.description().to_owned() ) ),
//            };
            let peer_ctx = PeerContext::new( signer, peer_auth.public_key, peer_auth.profile_id );
            Ok( (conn, peer_ctx) )
        } );
    Box::new(handshake_fut)
    //Box::new( future::err(ErrorToBeSpecified::TODO("unimplemented".to_owned())))
}
