use bytes::{Buf, BufMut, BytesMut, IntoBuf};
use std::mem;
use std::rc::Rc;

use futures::{future, Future};
use serde_json::{from_slice, to_vec};
//bincode::{deserialize, serialize};
use tokio_core::net::TcpStream;
use tokio_io::io;

use super::*;



#[derive(Deserialize, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
struct AuthenticationInfo
{
    profile_id: ProfileId,
    public_key: PublicKey,
}


pub fn temp_handshake_until_tls_is_implemented<R,W>(reader: R, writer: W, signer: Rc<Signer>)
    -> Box< Future<Item=(R, W, PeerContext), Error=Error> >
where R: std::io::Read + tokio_io::AsyncRead + 'static,
      W: std::io::Write + tokio_io::AsyncWrite + 'static
{
    debug!("Starting handshake with peer");
    let auth_info = AuthenticationInfo{
        profile_id: signer.profile_id().to_owned(), public_key: signer.public_key().to_owned() };

    let out_bytes = match to_vec(&auth_info) {
        Ok(data) => data,
        Err(e) => return Box::new( future::err( e.context(ErrorKind::TlsHandshakeFailed).into()) ),
    };
    let bufsize = out_bytes.len() as u32;

    let mut size_out_bytes = BytesMut::with_capacity( mem::size_of_val(&bufsize) );
    size_out_bytes.put_u32_le(bufsize);
    debug!("Sending serialized auth info of myself");

    let handshake_fut = io::write_all(writer, size_out_bytes)
        .and_then( move |(writer, _buf)| { io::write_all(writer, out_bytes) } )
        .and_then( move |(writer, _buf)|
        {
            debug!("Reading buffer size for peer info");
            let mut size_bytes = BytesMut::new();
            size_bytes.resize( mem::size_of_val(&bufsize), 0 );
            io::read_exact(reader, size_bytes)
                .map( |(reader, buf)| (reader, writer, buf) )
        } )
        .and_then( |(reader, writer, buf)|
        {
            debug!("Reading peer info, size: {:?}", buf);
            let size_in_bytes = buf.into_buf().get_u32_le();
            let mut in_bytes = BytesMut::new();
            in_bytes.resize(size_in_bytes as usize, 0);
            io::read_exact(reader, in_bytes)
                .map( |(reader, buf)| (reader, writer, buf) )
        } )        
        .and_then( |(reader, writer, buf)|
        {
            debug!("Processing peer info received");
            let peer_auth: AuthenticationInfo = from_slice(&buf)
                .map_err( |e| std::io::Error::new( std::io::ErrorKind::Other, e) )?;
            debug!("Received peer identity: {:?}", peer_auth);
            let peer_ctx = PeerContext::new( signer, peer_auth.public_key, peer_auth.profile_id );
            Ok( (reader, writer, peer_ctx) )
        } )
        .map_err(|err| err.context(ErrorKind::TlsHandshakeFailed).into()); 
    Box::new(handshake_fut)
}



pub fn temp_tcp_handshake_until_tls_is_implemented(socket: TcpStream, signer: Rc<Signer>)
    -> Box< Future<Item=(impl std::io::Read, impl std::io::Write, PeerContext), Error=Error> >
{
    use tokio_io::AsyncRead;

    match socket.set_nodelay(true) {
        Ok(_) => {},
        Err(e) => return Box::new( future::err(e.context(ErrorKind::TlsHandshakeFailed).into())),
    };

    let (reader, writer) = socket.split();
    temp_handshake_until_tls_is_implemented(reader, writer, signer)
}
