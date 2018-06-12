use bytes::{Buf, BufMut, BytesMut, IntoBuf};
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


pub fn temp_handshake_until_tls_is_implemented(socket: TcpStream, signer: Rc<Signer>)
    -> Box< Future<Item=(TcpStream, PeerContext), Error=ErrorToBeSpecified> >
{
    debug!("Starting handshake with peer");
    let auth_info = AuthenticationInfo{
        profile_id: signer.profile_id().to_owned(), public_key: signer.public_key().to_owned() };

    let out_bytes = match serialize(&auth_info) {
        Ok(data) => data,
        Err(e) => return Box::new( future::err( ErrorToBeSpecified::TODO( e.description().to_owned() ) ) ),
    };
    let bufsize = out_bytes.len() as u32;

    let mut size_out_bytes = BytesMut::with_capacity( mem::size_of_val(&bufsize) );
    size_out_bytes.put_u32_le(bufsize);
    debug!("Sending serialized auth info of myself");

    let handshake_fut = io::write_all(socket, size_out_bytes)
        .and_then( move |(socket, _buf)| { io::write_all(socket, out_bytes) } )
        .and_then( move |(socket, _buf)|
        {
            debug!("Reading buffer size for peer info");
            let mut size_bytes = BytesMut::new();
            size_bytes.resize( mem::size_of_val(&bufsize), 0 );
            io::read_exact(socket, size_bytes)
        } )
        .and_then( |(socket, buf)|
        {
            debug!("Reading peer info, size: {:?}", buf);
            let size_in_bytes = buf.into_buf().get_u32_le();
            let mut in_bytes = BytesMut::new();
            in_bytes.resize(size_in_bytes as usize, 0);
            io::read_exact(socket, in_bytes)
        } )
        .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )
        .and_then( |(socket, buf)|
        {
            debug!("Processing peer info received");
            let peer_auth: AuthenticationInfo = deserialize(&buf)
                .map_err( |e| ErrorToBeSpecified::TODO( e.description().to_owned() ) )?;
            debug!("Received peer identity: {:?}", peer_auth);
            let peer_ctx = PeerContext::new( signer, peer_auth.public_key, peer_auth.profile_id );
            Ok( (socket, peer_ctx) )
        } );
    Box::new(handshake_fut)
}
