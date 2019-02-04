use std::collections::HashMap;
use std::io::prelude::*;
use std::sync::mpsc;
use std::thread;

use log::*;

use crate::model::*;
use crate::messages::*;



// TODO should all operations below be async?
pub trait Profile
{
    fn id(&self) -> &ProfileId;
    fn links(&self) -> &[Link];
    fn metadata(&self) -> &HashMap<AttributeId,AttributeValue>;
    fn followers(&self) -> &[Link];

    fn create_link(&mut self, peer_profile: &ProfileId) -> Link;
    fn remove_link(&mut self, id: &LinkId);

    fn set_attribute(&mut self, key: AttributeId, value: AttributeValue);
    fn clear_attribute(&mut self, key: &AttributeId);

    //fn sign(&self, data: &[u8]) -> Signature;
    //fn get_signer(&self) -> Arc<Signer>;
}



pub fn run_rpc_network<R,W>(mut reader: R, mut writer: W)
    -> (mpsc::Sender<Envelope>, mpsc::Receiver<Envelope>)
where R: 'static + Read + Send,
      W: 'static + Write + Send
{
    // TODO Check if this bool is captured by reference and not copied/cloned into closures.
    //      Do we need an atomic bool or locking here instead?
    let mut stop_rpc = false;

    let (out_sender, out_receiver) = mpsc::channel();
    let (in_sender, in_receiver) = mpsc::channel(); // TOOD is this really needed?

    let req_send_thread_join = thread::spawn( move ||
    {
        for envelope in out_receiver {
            if stop_rpc
                { break; }

            //debug!("Sending envelope {:?}", envelope);
            // TODO consider proper error handling instead of dumbing all down to Option
            let fwd_req = rmp_serde::encode::to_vec_named(&envelope).ok()
                .and_then( |bytes| {
                    debug!("Sending bytes {:?}", bytes);
//                    use std::fs::File;
//                    let mut binfile = File::create("/tmp/messagepack_bytes.dat").unwrap();
//                    binfile.write_all(&bytes).unwrap();
                    writer.write_all(&bytes).ok()
                } );
            if let None = fwd_req {
                warn!("Failed to send out request, stop");
                stop_rpc = true;
                break;
            }
        }
    } );

    let resp_recv_thread_join = thread::spawn( move ||
    {
        while !stop_rpc {
            // TODO read into buffer first and deserialize envelopes from there
            let msg_envelope_res : Result<Envelope,rmp_serde::decode::Error> = rmp_serde::from_read(&mut reader);
            let msg_envelope = match msg_envelope_res {
                Ok(envelope) => envelope,
                Err(e) => {
                    warn!("Failed to read message envelope, stop: {}", e);
                    stop_rpc = true;
                    break;
                }
            };
            if let Err(e) = in_sender.send(msg_envelope) {
                warn!("Failed to sned incoming message envelope further, stop: {}", e);
                stop_rpc = true;
                break;
            }
        }

//        const READ_BUFFER_SIZE : u64 = 64;
//        const MAX_MESSAGE_SIZE : u64 = 1024;
//
//        let mut msg_buf = Vec::new();
//        msg_buf.resize( (MAX_MESSAGE_SIZE + READ_BUFFER_SIZE) as usize, 0 );
//        let mut msg_cursor = Cursor::new(&mut msg_buf);
//
//        let mut read_buf = [0u8; 64];
//        loop {
//            let read_res = reader.read(&mut read_buf);
//            let bytes_read = match read_res
//            {
//                Ok(bytes_read) => bytes_read,
//                Err(e) => {
//                    warn!("Failed to read message, stop reading");
//                    break;
//                },
//            };
//            if let Err(e) = msg_cursor.write_all( &read_buf[..bytes_read] ) {
//                warn!("Failed to fill up message buffer with bytes read, stop reading");
//                break;
//            }
//            if msg_cursor.position() >= MAX_MESSAGE_SIZE {
//                // TODO
//            }
//            // TODO
//        }
    } );

    // Show explicit intention to detach from these threads
    drop(req_send_thread_join);
    drop(resp_recv_thread_join);

    (out_sender, in_receiver)
}


pub struct RpcProfile
{
    id: ProfileId,
    sender: mpsc::Sender<Envelope>,
}


impl RpcProfile
{
    pub fn new(id: ProfileId, sender: mpsc::Sender<Envelope>) -> Self
        { Self{ id, sender } }
}


// TODO probably all methods should return Result with some Error type instead
impl Profile for RpcProfile
{
    fn id(&self) -> &ProfileId { &self.id }
    fn links(&self) -> &[Link]  { unimplemented!() }
    fn metadata(&self) -> &HashMap<AttributeId,AttributeValue>  { unimplemented!() }
    fn followers(&self) -> &[Link]  { unimplemented!() }

    fn create_link(&mut self, peer_profile: &ProfileId) -> Link
    {
        // TODO kill all unwraps
        let params = AddEdgeParams{ source: self.id().to_owned(), target: peer_profile.to_owned() };
        let request = Request::new("add_edge", params);
        let send_res = self.sender.send( Envelope::from(request).unwrap() );
        send_res.unwrap();
        // TODO wait for response and fill fields from that
        thread::sleep( std::time::Duration::from_secs(5) );
        Link{ id: ProfileId(vec![]), peer_profile: peer_profile.to_owned() }
    }
    fn remove_link(&mut self, id: &LinkId)  { unimplemented!() }

    fn set_attribute(&mut self, key: AttributeId, value: AttributeValue)
    {
        // TODO kill all unwraps
        let params = SetAttributeParams{ key, value };
        let request = Request::new("set_attribute", params);
        let send_res = self.sender.send( Envelope::from(request).unwrap() );
        send_res.unwrap();
        thread::sleep( std::time::Duration::from_secs(5) );
    }
    fn clear_attribute(&mut self, key: &AttributeId)  { unimplemented!() }
}
