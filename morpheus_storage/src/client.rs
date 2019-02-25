use std::cell::RefCell;
use std::collections::HashMap;
use std::io::prelude::*;
use std::rc::Rc;

use failure::{bail, err_msg, Fallible};
use log::*;

use crate::messages::*;
use crate::model::*;

const MORPHEUS_HANDLER: &str = "osg";
const RESPONSE_CODE_OK: u32 = 0;

pub type AttributeMap = HashMap<AttributeId, AttributeValue>;

// TODO should all operations below be async?
pub trait ProfileRepository {
    fn get(&self, id: &ProfileId) -> Option<ProfilePtr>;
    fn create(&mut self, id: &ProfileId) -> Fallible<ProfilePtr>;
    // TODO what does this mean? Purge related metadata from local storage plus don't show it in the list,
    //      or maybe also delete all links/follows with other profiles
    fn remove(&mut self, id: &ProfileId) -> Fallible<()>;
}

// TODO should all operations below be async?
pub trait Profile {
    fn id(&self) -> ProfileId;
    // TODO naming is inconsistent, metadata vs attribute, should use only one of them
    fn metadata(&self) -> Fallible<AttributeMap>;
    fn links(&self) -> Fallible<Vec<Link>>;
    fn followers(&self) -> Fallible<Vec<Link>>;

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link>;
    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()>;

    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()>;
    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()>;

    //fn sign(&self, data: &[u8]) -> Signature;
    //fn get_signer(&self) -> Arc<Signer>;
}

pub type ProfilePtr = Rc<RefCell<Profile>>;
pub type RpcPtr<R, W> = Rc<RefCell<MsgPackRpc<R, W>>>;

pub struct RpcProfile<R, W> {
    id: ProfileId,
    rpc: RpcPtr<R, W>,
}

impl<R, W> RpcProfile<R, W>
where
    R: 'static + Read,
    W: 'static + Write,
{
    pub fn new(id: &ProfileId, rpc: RpcPtr<R, W>) -> Self {
        Self {
            id: id.to_owned(),
            rpc,
        }
    }

    fn send_request<T>(&self, method: &str, params: T) -> Fallible<Response>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        self.rpc.borrow_mut().send_request(method, params)
    }

    pub fn get_node_attribute(&self, key: AttributeId) -> Fallible<Vec<u8>> {
        let params = GetNodeAttributeParams {
            id: self.id.to_owned(),
            key: key,
        };
        let response = self.send_request("get_node_attribute", params)?;
        let attr_val = response
            .reply
            .ok_or_else(|| err_msg("Server returned no reply content for query"))
            .and_then(|resp_val| match resp_val {
                rmpv::Value::Binary(bin) => Ok(bin),
                _ => bail!("Server returned unexpected attribute type"),
            })?;
        Ok(attr_val)
    }

    pub fn set_node_attribute(&self, key: AttributeId, value: Vec<u8>) -> Fallible<()> {
        let params = SetNodeAttributeParams {
            id: self.id.to_owned(),
            key,
            value,
        };
        let _response = self.send_request("set_node_attribute", params)?;
        Ok(())
    }

    pub fn clear_node_attribute(&self, key: String) -> Fallible<()> {
        let params = ClearNodeAttributeParams {
            id: self.id.to_owned(),
            key,
        };
        let _response = self.send_request("clear_node_attribute", params)?;
        Ok(())
    }

    const OPEN_SOCIAL_GRAPH_ATTRIBUTE: &'static str = "osg";

    pub fn get_osg_attribute_map(&self) -> Fallible<AttributeMap> {
        let attr_map_bin = self.get_node_attribute(Self::OPEN_SOCIAL_GRAPH_ATTRIBUTE.to_owned())?;
        let attributes = serde_json::from_slice(&attr_map_bin)?;
        Ok(attributes)
    }

    pub fn set_osg_attribute_map(&self, attributes: AttributeMap) -> Fallible<()> {
        let attr_map_bin = serde_json::to_vec(&attributes)?;
        self.set_node_attribute(Self::OPEN_SOCIAL_GRAPH_ATTRIBUTE.to_owned(), attr_map_bin)
    }

    // TOOD either source or target should be self.id but how to differentiate incoming and outgoing edges?
    //      Maybe separate calls?
    // TODO consider if we also should hide all attributes behind a separate "namespace" key
    //      and Json-like document as for node attributes
    pub fn get_edge_attribute(
        &self,
        source: ProfileId,
        target: ProfileId,
        key: AttributeId,
    ) -> Fallible<Vec<u8>> {
        let params = GetEdgeAttributeParams {
            source,
            target,
            key,
        };
        let response = self.send_request("get_edge_attribute", params)?;
        let attr_val = response
            .reply
            .ok_or_else(|| err_msg("Server returned no reply content for query"))
            .and_then(|resp_val| match resp_val {
                rmpv::Value::Binary(bin) => Ok(bin),
                _ => bail!("Server returned unexpected attribute type"),
            })?;
        Ok(attr_val)
    }

    pub fn set_edge_attribute(
        &self,
        source: ProfileId,
        target: ProfileId,
        key: AttributeId,
        value: Vec<u8>,
    ) -> Fallible<()> {
        let params = SetEdgeAttributeParams {
            source,
            target,
            key,
            value,
        };
        let _response = self.send_request("set_edge_attribute", params)?;
        Ok(())
    }

    pub fn clear_edge_attribute(
        &self,
        source: ProfileId,
        target: ProfileId,
        key: AttributeId,
    ) -> Fallible<()> {
        let params = ClearEdgeAttributeParams {
            source,
            target,
            key,
        };
        let _response = self.send_request("clear_edge_attribute", params)?;
        Ok(())
    }
}

impl<R, W> Profile for RpcProfile<R, W>
where
    R: 'static + Read,
    W: 'static + Write,
{
    fn id(&self) -> ProfileId {
        self.id.clone()
    }

    fn metadata(&self) -> Fallible<AttributeMap> {
        self.get_osg_attribute_map()
    }

    fn links(&self) -> Fallible<Vec<Link>> {
        let params = ListOutEdgesParams {
            id: self.id().clone(),
        };
        let response = self.send_request("list_outedges", params)?;
        let reply_val = response
            .reply
            .ok_or_else(|| err_msg("Server returned no reply content for query"))?;
        let reply: ListOutEdgesReply = rmpv::ext::from_value(reply_val)?;
        let links = reply
            .into_iter()
            .map(|peer_profile| Link { peer_profile })
            .collect();
        Ok(links)
    }

    fn followers(&self) -> Fallible<Vec<Link>> {
        let params = ListInEdgesParams {
            id: self.id().clone(),
        };
        let response = self.send_request("list_inedges", params)?;
        let reply_val = response
            .reply
            .ok_or_else(|| err_msg("Server returned no reply content for query"))?;
        let reply: ListInEdgesReply = rmpv::ext::from_value(reply_val)?;
        let followers = reply
            .into_iter()
            .map(|peer_profile| Link { peer_profile })
            .collect();
        Ok(followers)
    }

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link> {
        let params = AddEdgeParams {
            source: self.id().to_owned(),
            target: peer_profile.to_owned(),
        };
        let _response = self.send_request("add_edge", params)?;
        Ok(Link {
            peer_profile: peer_profile.to_owned(),
        })
    }

    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()> {
        let params = RemoveEdgeParams {
            source: self.id().to_owned(),
            target: peer_profile.to_owned(),
        };
        let _response = self.send_request("remove_edge", params)?;
        Ok(())
    }

    // TODO get and set_attr() consist of two remote operations. If any of those fail,
    //      wallet and storage backend states might easily get unsynchronized.
    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()> {
        let mut attr_map = self.get_osg_attribute_map()?;
        attr_map.insert(key.to_owned(), value.to_owned());
        self.set_osg_attribute_map(attr_map)
    }

    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()> {
        let mut attr_map = self.get_osg_attribute_map()?;
        attr_map.remove(key);
        self.set_osg_attribute_map(attr_map)
    }
}

pub struct MsgPackRpc<R, W> {
    reader: R,
    writer: W,
    next_rid: MessageId,
}

impl<R, W> MsgPackRpc<R, W>
where
    R: 'static + Read,
    W: 'static + Write,
{
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            next_rid: 1,
        }
    }

    // TODO this should probably return a different error type to differentiate
    //      between different errors returned by the server.
    //      They are currently defined as the following in the server:
    //
    //  enum class error_code : uint8_t {
    //    ok                  = 0,
    //    key_not_found       = 1,
    //    key_already_exists  = 2,
    //    attribute_not_found = 3,
    //    source_not_found    = 4,
    //    target_not_found    = 5,
    //  };
    pub fn send_request<T>(&mut self, method: &str, params: T) -> Fallible<Response>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        let req_rid = self.next_rid;
        self.next_rid += 1;
        let request = Request::new(req_rid, method, params);
        trace!("Sending request {:?}", request);

        let req_envelope = Envelope::from(MORPHEUS_HANDLER, request)?;
        let req_envelope_bytes = rmp_serde::encode::to_vec_named(&req_envelope)?;
        trace!("Sending bytes {:?}", req_envelope_bytes);

        // let mut req_file = std::fs::File::create("/tmp/messagepack_bytes.dat")?;
        // req_file.write_all(&req_envelope_bytes)?;
        self.writer.write_all(&req_envelope_bytes)?;

        trace!("Request sent, reading response");
        let resp_envelope: Envelope = rmp_serde::from_read(&mut self.reader)?;
        if resp_envelope.target != MORPHEUS_HANDLER {
            bail!(
                "Unexpected target of response message: {}",
                resp_envelope.target
            );
        }

        let response: Response = rmp_serde::from_slice(&resp_envelope.payload)?;
        if response.rid != req_rid {
            bail!(
                "Expected response to request {}, Got response for {}",
                req_rid,
                response.rid
            );
        }

        if response.code != RESPONSE_CODE_OK {
            bail!(
                "Got error response with code {}, description: {}",
                response.code,
                response.description.unwrap_or_else(|| "None".to_owned())
            );
        }

        trace!("Got response {:?}", response);
        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::model::ProfileId;
    use crate::store::RpcProfileRepository;
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn test_server_calls() -> Fallible<()> {
        let addr = "127.0.0.1:6161".parse()?;
        let timeout = Duration::from_secs(5);
        let mut store = RpcProfileRepository::new(&addr, timeout)?;

        let nodes = store.list_nodes()?;
        assert_eq!(nodes.len(), 0);

        let my_id = ProfileId::from_str("IezbeWGSY2dqcUBqT8K7R14xr")?;
        let me = store.create(&my_id)?;
        let peer_id = ProfileId::from_str("Iez25N5WZ1Q6TQpgpyYgiu9gTX")?;
        let peer = store.create(&peer_id)?;

        let nodes = store.list_nodes()?;
        assert_eq!(nodes.len(), 2);
        assert_eq!(me.borrow().links()?.len(), 0);
        assert_eq!(me.borrow().followers()?.len(), 0);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(peer.borrow().followers()?.len(), 0);

        let link = me.borrow_mut().create_link(&peer_id)?;
        assert_eq!(nodes.len(), 2);
        assert_eq!(link.peer_profile, peer_id);
        assert_eq!(me.borrow().followers()?.len(), 0);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(me.borrow().links()?.len(), 1);
        assert_eq!(me.borrow().links()?[0].peer_profile, peer_id);
        assert_eq!(peer.borrow().followers()?[0].peer_profile, my_id);

        me.borrow_mut().remove_link(&peer_id)?;
        assert_eq!(nodes.len(), 2);
        assert_eq!(me.borrow().links()?.len(), 0);
        assert_eq!(me.borrow().followers()?.len(), 0);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(peer.borrow().followers()?.len(), 0);

        let attr_id = "1 2 3".to_owned();
        let attr_val = "one two three".to_owned();
        assert_eq!(me.borrow().metadata()?.len(), 0);
        assert_eq!(peer.borrow().metadata()?.len(), 0);
        me.borrow_mut().set_attribute(&attr_id, &attr_val)?;
        assert_eq!(me.borrow().metadata()?.len(), 1);
        assert_eq!(me.borrow().metadata()?.get(&attr_id), Some(&attr_val));
        assert_eq!(peer.borrow().metadata()?.len(), 0);
        me.borrow_mut().clear_attribute(&attr_id)?;
        assert_eq!(me.borrow().metadata()?.len(), 0);
        assert_eq!(me.borrow().metadata()?.len(), 0);

        assert_eq!(nodes.len(), 2);
// TODO consider if we need removing profiles or keep it unimplemented
//        store.remove(&my_id)?;
//        store.remove(&peer_id)?;
//        assert_eq!(nodes.len(), 0);

        Ok(())
    }
}
