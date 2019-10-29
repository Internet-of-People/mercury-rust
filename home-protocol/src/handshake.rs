use std::mem;
use std::rc::Rc;

//bincode::{deserialize, serialize};
use bytes::{Buf, BufMut, BytesMut, IntoBuf};
use failure::{ensure, Fallible};
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use futures_tokio_compat::Compat;
use serde_json::{from_slice, to_vec};
use tokio::net::TcpStream;
//use x25519_dalek::diffie_hellman;

use crate::*;

#[derive(Deserialize, Clone, Debug, Eq, PartialEq, Serialize)]
struct AuthenticationInfo {
    public_key: PublicKey,
    // TODO this might not be needed, can be deduced from public_key
    profile_id: ProfileId,
}

impl AuthenticationInfo {
    fn new(signer: &dyn Signer) -> Self {
        Self { profile_id: signer.profile_id().to_owned(), public_key: signer.public_key() }
    }
}

async fn exchange_identities<'a, R, W>(
    reader: &'a mut R,
    writer: &'a mut W,
    signer: Rc<dyn Signer>,
) -> Fallible<AuthenticationInfo>
where
    R: AsyncRead + Unpin + 'a,
    W: AsyncWrite + Unpin + 'a,
{
    debug!("Starting handshake with peer");
    let auth_info = AuthenticationInfo::new(signer.as_ref());
    let mut out_bytes = to_vec(&auth_info).map_err_dh()?;
    let bufsize = out_bytes.len() as u32;
    let mut size_out_bytes = BytesMut::with_capacity(mem::size_of_val(&bufsize));
    size_out_bytes.put_u32_le(bufsize);

    trace!("Sending auth info of myself: {:?}", auth_info);
    AsyncWriteExt::write_all(writer, &mut size_out_bytes).await.map_err_dh()?;
    AsyncWriteExt::write_all(writer, &mut out_bytes).await.map_err_dh()?;

    trace!("Reading buffer size for peer info");
    let mut size_bytes = BytesMut::new();
    size_bytes.resize(mem::size_of_val(&bufsize), 0);
    AsyncReadExt::read_exact(reader, &mut size_bytes).await.map_err_dh()?;

    trace!("Reading peer info, size: {:?}", size_bytes);
    let size_in_bytes = size_bytes.into_buf().get_u32_le();
    ensure!(size_in_bytes > 8096, "Authentication packet too long");
    let mut in_bytes = BytesMut::new();
    in_bytes.resize(size_in_bytes as usize, 0);
    AsyncReadExt::read_exact(reader, &mut in_bytes).await.map_err_dh()?;

    trace!("Processing peer info received");
    let peer_auth: AuthenticationInfo = from_slice(&in_bytes).map_err_dh()?;

    trace!("Received peer identity: {:?}", peer_auth);
    Ok(peer_auth)
}

pub fn tcpstream_to_reader_writer(
    socket: TcpStream,
) -> std::io::Result<(ReadHalf<Compat<TcpStream>>, WriteHalf<Compat<TcpStream>>)> {
    socket.set_nodelay(true)?;
    let compat = Compat::new(socket);
    Ok(compat.split())
}

pub async fn ecdh_handshake<'a, R, W>(
    reader: &'a mut R,
    writer: &'a mut W,
    signer: Rc<dyn Signer>,
) -> Fallible<PeerContext>
where
    R: AsyncRead + Unpin + 'a,
    W: AsyncWrite + Unpin + 'a,
{
    let peer_auth = exchange_identities(reader, writer, signer.clone()).await?;
    //let _my_secret_key: [u8; 32] = Default::default(); // TODO how to get secret key as bytes, especially when HW wallet support is planned?

    //let mut peer_pubkey: [u8; 32] = Default::default();
    //peer_pubkey.copy_from_slice(peer_auth.public_key.to_bytes());

    // TODO shadow reader and writer with ones using the shared secret to do symmetric en/decryption
    //      and return them with a proper peercontext
    // let shared_secret = diffie_hellman(&my_secret_key, &peer_ed25519_pubkey);

    let peer_ctx = PeerContext::new(signer, peer_auth.public_key);
    Ok(peer_ctx)
}

pub async fn temporary_unsafe_handshake_until_diffie_hellman_done<'a, R, W>(
    reader: &'a mut R,
    writer: &'a mut W,
    signer: Rc<dyn Signer>,
) -> Fallible<PeerContext>
where
    R: AsyncRead + Unpin + 'a,
    W: AsyncWrite + Unpin + 'a,
{
    let peer_auth = exchange_identities(reader, writer, signer.clone()).await?;
    warn!("No proper peer validation was performed, safety is ignored");
    let peer_ctx = PeerContext::new(signer, peer_auth.public_key);
    debug!("Handshake succeeded");
    Ok(peer_ctx)
}

pub async fn tcp_ecdh_handshake(
    socket: TcpStream,
    signer: Rc<dyn Signer>,
) -> Fallible<(PeerContext, ReadHalf<Compat<TcpStream>>, WriteHalf<Compat<TcpStream>>)> {
    let (mut r, mut w) = tcpstream_to_reader_writer(socket).map_err_dh()?;
    let ctx = ecdh_handshake(&mut r, &mut w, signer).await?;
    Ok((ctx, r, w))
}

pub async fn temporary_unsafe_tcp_handshake_until_diffie_hellman_done(
    socket: TcpStream,
    signer: Rc<dyn Signer>,
) -> Fallible<(PeerContext, ReadHalf<Compat<TcpStream>>, WriteHalf<Compat<TcpStream>>)> {
    let (mut r, mut w) = tcpstream_to_reader_writer(socket).map_err_dh()?;
    let ctx = temporary_unsafe_handshake_until_diffie_hellman_done(&mut r, &mut w, signer).await?;
    Ok((ctx, r, w))
}
