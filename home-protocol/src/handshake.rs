use bytes::{Buf, BufMut, BytesMut, IntoBuf};
use std::mem;
use std::rc::Rc;

//bincode::{deserialize, serialize};
use failure::Fail;
use futures::prelude::*;
use serde_json::{from_slice, to_vec};
use tokio_core::net::TcpStream;
use tokio_io::io;
//use x25519_dalek::diffie_hellman;

use crate::*;
use crate::error::*;



#[derive(Deserialize, Clone, Debug, Eq, Hash, PartialEq, PartialOrd, Serialize)]
struct AuthenticationInfo
{
    profile_id: ProfileId,
    public_key: PublicKey,
}


fn exchange_identities<R,W>(reader: R, writer: W, signer: Rc<Signer>)
    -> Box< Future<Item=(R, W, AuthenticationInfo), Error=Error> >
where R: std::io::Read + tokio_io::AsyncRead + 'static,
      W: std::io::Write + tokio_io::AsyncWrite + 'static
{
    debug!("Starting handshake with peer");
    let auth_info = AuthenticationInfo{
        profile_id: signer.profile_id().to_owned(), public_key: signer.public_key().to_owned() };

    let out_bytes = match to_vec(&auth_info) {
        Ok(data) => data,
        Err(e) => return Box::new( Err( e.context( ErrorKind::DiffieHellmanHandshakeFailed).into() ).into_future() ),
    };
    let bufsize = out_bytes.len() as u32;

    let mut size_out_bytes = BytesMut::with_capacity( mem::size_of_val(&bufsize) );
    size_out_bytes.put_u32_le(bufsize);
    trace!("Sending auth info of myself: {:?}", auth_info);

    let exch_fut = io::write_all(writer, size_out_bytes)
        .and_then( move |(writer, _buf)| { io::write_all(writer, out_bytes) } )
        .and_then( move |(writer, _buf)|
        {
            trace!("Reading buffer size for peer info");
            let mut size_bytes = BytesMut::new();
            size_bytes.resize( mem::size_of_val(&bufsize), 0 );
            io::read_exact(reader, size_bytes)
                .map( |(reader, buf)| (reader, writer, buf) )
        } )
        .and_then( |(reader, writer, buf)|
        {
            trace!("Reading peer info, size: {:?}", buf);
            let size_in_bytes = buf.into_buf().get_u32_le();
            if size_in_bytes > 8096 {
                let err = Err( std::io::Error::from(std::io::ErrorKind::ConnectionAborted) );
                return Box::new( err.into_future() ) as Box<Future<Item=_,Error=_>>
            }
            let mut in_bytes = BytesMut::new();
            in_bytes.resize(size_in_bytes as usize, 0);
            let res_fut = io::read_exact(reader, in_bytes)
                .map( |(reader, buf)| (reader, writer, buf) );
            Box::new(res_fut)
        } )
        .and_then( |(reader, writer, buf)|
        {
            trace!("Processing peer info received");
            let peer_auth: AuthenticationInfo = from_slice(&buf)
                .map_err( |e| std::io::Error::new( std::io::ErrorKind::Other, e) )?;
            trace!("Received peer identity: {:?}", peer_auth);
            Ok( (reader, writer, peer_auth) )
        } )
        .map_err( |e| e.context(ErrorKind::DiffieHellmanHandshakeFailed).into() );
    Box::new(exch_fut)
}



pub fn tcpstream_to_reader_writer(socket: TcpStream)
    -> Result<(impl std::io::Read + tokio_io::AsyncRead,
               impl std::io::Write + tokio_io::AsyncWrite), std::io::Error>
{
    use tokio_io::AsyncRead;
    socket.set_nodelay(true)?;
    Ok( socket.split() )
}



pub fn ecdh_handshake<R,W>(reader: R, writer: W, signer: Rc<Signer>)
    -> Box< Future<Item=(R, W, PeerContext), Error=Error> >
where R: std::io::Read + tokio_io::AsyncRead + 'static,
      W: std::io::Write + tokio_io::AsyncWrite + 'static
{
    let ecdh_fut = exchange_identities( reader, writer, signer.clone() )
        .and_then( |(reader, writer, peer_auth)|
        {
            let _my_secret_key : [u8; 32] = Default::default(); // TODO how to get secret key as bytes, especially when HW wallet support is planned?

            if peer_auth.public_key.0.len() != 32 {
                return Err( ErrorKind::DiffieHellmanHandshakeFailed.into() );
            }
            let mut peer_ed25519_pubkey : [u8; 32] = Default::default();
            peer_ed25519_pubkey.copy_from_slice( peer_auth.public_key.0.as_slice() );

            // TODO shadow reader and writer with ones using the shared secret to do symmetric en/decryption
            //      and return them with a proper peercontext
            // let shared_secret = diffie_hellman(&my_secret_key, &peer_ed25519_pubkey);

            let peer_ctx = PeerContext::new( signer, peer_auth.public_key, peer_auth.profile_id );
            Ok( (reader, writer, peer_ctx) )
        } );
    Box::new(ecdh_fut)
}


pub fn temporary_unsafe_handshake_until_diffie_hellman_done<R,W>(reader: R, writer: W, signer: Rc<Signer>)
    -> Box< Future<Item=(R, W, PeerContext), Error=Error> >
where R: std::io::Read + tokio_io::AsyncRead + 'static,
      W: std::io::Write + tokio_io::AsyncWrite + 'static
{
    let handshake_fut = exchange_identities( reader, writer, signer.clone() )
        .map_err( |err| err.context(ErrorKind::DiffieHellmanHandshakeFailed).into() )
        .and_then( |(reader, writer, peer_auth)|
        {
            warn!("No proper peer validation was performed, safety is ignored");
            let peer_ctx = PeerContext::new( signer, peer_auth.public_key, peer_auth.profile_id );
            debug!("Handshake succeeded");
            Ok( (reader, writer, peer_ctx) )
        } )
        ;
    Box::new(handshake_fut)
}



pub fn tcp_ecdh_handshake(socket: TcpStream, signer: Rc<Signer>)
    -> Box< Future<Item=(impl std::io::Read + tokio_io::AsyncRead,
           impl std::io::Write + tokio_io::AsyncWrite, PeerContext), Error=Error> >
{
    let res_fut = tcpstream_to_reader_writer(socket).into_future()
        .map_err( |e| e.context(ErrorKind::DiffieHellmanHandshakeFailed).into() )
        .and_then( |(reader,writer)| ecdh_handshake(reader, writer, signer) );
    Box::new(res_fut)
}


pub fn temporary_unsafe_tcp_handshake_until_diffie_hellman_done(socket: TcpStream, signer: Rc<Signer>)
    -> Box< Future<Item=(impl std::io::Read + tokio_io::AsyncRead,
           impl std::io::Write + tokio_io::AsyncWrite, PeerContext), Error=Error> >
{
    let res_fut = tcpstream_to_reader_writer(socket).into_future()
        .map_err( |e| e.context(ErrorKind::DiffieHellmanHandshakeFailed).into() )
        .and_then( |(reader,writer)| temporary_unsafe_handshake_until_diffie_hellman_done(reader, writer, signer) );
    Box::new(res_fut)
}
