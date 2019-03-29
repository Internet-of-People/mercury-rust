use std::cell::RefCell;
use std::io::prelude::*;
use std::rc::Rc;

use enum_repr::EnumRepr;
use failure::{bail, err_msg, Fail, Fallible};
use log::*;

use crate::messages::*;
use osg::model::*;
use osg::profile::Profile;

const MORPHEUS_HANDLER: &str = "osg";
const RESPONSE_CODE_OK: u8 = 0;

#[derive(Clone, Debug, Eq, Fail, Hash, PartialEq)]
pub enum RpcError {
    #[fail(display = "Server returned no response for request")]
    NoResponse,
    #[fail(
        display = "Expected response to request {}, Got response for {}",
        request_id, response_id
    )]
    UnexpectedResponseId { request_id: MessageId, response_id: MessageId },
    #[fail(display = "Server returned unexpected attribute type")]
    UnexpectedAttributeType,
    #[fail(display = "Unexpected target of response message: {}", target)]
    UnexpectedTarget { target: String },
    #[fail(display = "Morpheus error {}: {}", code, description)]
    Morpheusd { code: MorpheusdError, description: String },
    #[fail(display = "Local RPC error: {}", msg)]
    Unknown { msg: String },
    #[fail(display = "Unknown Morpheus error {}: {}", code, description)]
    UnknownMorpheusd { code: u8, description: String },
}

#[EnumRepr(type = "u8")]
#[derive(Clone, Debug, Eq, Fail, Hash, Ord, PartialEq, PartialOrd)]
pub enum MorpheusdError {
    #[fail(display = "Profile identifier not found")]
    KeyNotFound = 1,
    #[fail(display = "Profile was already existing")]
    KeyAlreadyExists = 2,
    #[fail(display = "Attribute was not found on that profile")]
    AttributeNotFound = 3,
    #[fail(display = "Internal server error")]
    InternalServerError = 4,
}

pub trait FallibleExtension<T> {
    fn key_not_existed_or_else<F>(self, fallback: F) -> Fallible<T>
    where
        F: FnOnce() -> Fallible<T>;
}

impl<T> FallibleExtension<T> for Fallible<T> {
    fn key_not_existed_or_else<F>(self, fallback: F) -> Fallible<T>
    where
        F: FnOnce() -> Fallible<T>,
    {
        if let Err(e) = &self {
            if let Some(rpc) = e.downcast_ref::<RpcError>() {
                if let RpcError::Morpheusd { code: MorpheusdError::KeyAlreadyExists, .. } = rpc {
                    return fallback();
                }
            }
        }
        self
    }
}

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
        Self { id: id.to_owned(), rpc }
    }

    fn send_request<T>(&self, method: &str, params: T) -> Fallible<Response>
    where
        T: serde::Serialize + std::fmt::Debug,
    {
        self.rpc.borrow_mut().send_request(method, params)
    }

    pub fn get_node_attribute(&self, key: AttributeId) -> Fallible<Vec<u8>> {
        let params = GetNodeAttributeParams { id: self.id.to_owned(), key };
        let response = self.send_request("get_node_attribute", params)?;
        let attr_val =
            response.reply.ok_or_else(|| RpcError::NoResponse.into()).and_then(|resp_val| {
                match resp_val {
                    rmpv::Value::Binary(bin) => Ok(bin),
                    _ => Err(failure::Error::from(RpcError::UnexpectedAttributeType)),
                }
            })?;
        Ok(attr_val)
    }

    pub fn set_node_attribute(&self, key: AttributeId, value: Vec<u8>) -> Fallible<()> {
        let params = SetNodeAttributeParams { id: self.id.to_owned(), key, value };
        let _response = self.send_request("set_node_attribute", params)?;
        Ok(())
    }

    pub fn clear_node_attribute(&self, key: String) -> Fallible<()> {
        let params = ClearNodeAttributeParams { id: self.id.to_owned(), key };
        let _response = self.send_request("clear_node_attribute", params)?;
        Ok(())
    }

    const VERSION_ATTRIBUTE: &'static str = "version";
    const OPEN_SOCIAL_GRAPH_ATTRIBUTE: &'static str = "osg";

    pub fn get_osg_attribute_map(&self) -> Fallible<AttributeMap> {
        let attr_map_bin = self.get_node_attribute(Self::OPEN_SOCIAL_GRAPH_ATTRIBUTE.to_owned())?;
        let attributes = serde_json::from_slice(&attr_map_bin)?;
        Ok(attributes)
    }

    pub fn set_osg_attribute_map(&self, attributes: &AttributeMap) -> Fallible<()> {
        let attr_map_bin = serde_json::to_vec(attributes)?;
        self.set_node_attribute(Self::OPEN_SOCIAL_GRAPH_ATTRIBUTE.to_owned(), attr_map_bin)
    }

    // TODO either source or target should be self.id but how to differentiate incoming and outgoing edges?
    //      Maybe separate calls?
    // TODO consider if we also should hide all attributes behind a separate "namespace" key
    //      and Json-like document as for node attributes
    pub fn get_edge_attribute(
        &self,
        source: ProfileId,
        target: ProfileId,
        key: AttributeId,
    ) -> Fallible<Vec<u8>> {
        let params = GetEdgeAttributeParams { source, target, key };
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
        let params = SetEdgeAttributeParams { source, target, key, value };
        let _response = self.send_request("set_edge_attribute", params)?;
        Ok(())
    }

    pub fn clear_edge_attribute(
        &self,
        source: ProfileId,
        target: ProfileId,
        key: AttributeId,
    ) -> Fallible<()> {
        let params = ClearEdgeAttributeParams { source, target, key };
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

    fn version(&self) -> Fallible<Version> {
        let version_rmp = self.get_node_attribute(Self::VERSION_ATTRIBUTE.to_owned())?;
        let version: Version = rmp_serde::from_slice(&version_rmp)?;
        Ok(version)
    }

    fn attributes(&self) -> Fallible<AttributeMap> {
        self.get_osg_attribute_map()
    }

    fn links(&self) -> Fallible<Vec<Link>> {
        let params = ListOutEdgesParams { id: self.id().clone() };
        let response = self.send_request("list_outedges", params)?;
        let reply_val =
            response.reply.ok_or_else(|| err_msg("Server returned no reply content for query"))?;
        let reply: ListOutEdgesReply = rmpv::ext::from_value(reply_val)?;
        let links = reply.into_iter().map(|peer_profile| Link { peer_profile }).collect();
        Ok(links)
    }

    fn set_version(&mut self, version: Version) -> Fallible<()> {
        //let version = self.version()? + 1;
        let version_rmp = rmp_serde::to_vec(&version)?;
        self.set_node_attribute(Self::VERSION_ATTRIBUTE.to_owned(), version_rmp)
    }

    fn create_link(&mut self, peer_profile: &ProfileId) -> Fallible<Link> {
        let params =
            AddEdgeParams { source: self.id().to_owned(), target: peer_profile.to_owned() };
        self.send_request("add_edge", params).map(|_r| ()).key_not_existed_or_else(|| Ok(()))?;
        Ok(Link { peer_profile: peer_profile.to_owned() })
    }

    fn remove_link(&mut self, peer_profile: &ProfileId) -> Fallible<()> {
        let params =
            RemoveEdgeParams { source: self.id().to_owned(), target: peer_profile.to_owned() };
        let _response = self.send_request("remove_edge", params)?;
        Ok(())
    }

    // TODO get and set_attr() consist of two remote operations. If any of those fail,
    //      wallet and storage backend states might easily get unsynchronized.
    fn set_attribute(&mut self, key: &AttributeId, value: &AttributeValue) -> Fallible<()> {
        let mut attr_map = self.get_osg_attribute_map()?;
        attr_map.insert(key.to_owned(), value.to_owned());
        self.set_osg_attribute_map(&attr_map)
    }

    fn clear_attribute(&mut self, key: &AttributeId) -> Fallible<()> {
        let mut attr_map = self.get_osg_attribute_map()?;
        attr_map.remove(key);
        self.set_osg_attribute_map(&attr_map)
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
        Self { reader, writer, next_rid: 1 }
    }

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
        let target = resp_envelope.target;
        if target != MORPHEUS_HANDLER {
            return Err(RpcError::UnexpectedTarget { target }.into());
        }

        let response: Response = rmp_serde::from_slice(&resp_envelope.payload)?;
        if response.rid != req_rid {
            return Err(RpcError::UnexpectedResponseId {
                request_id: req_rid,
                response_id: response.rid,
            }
            .into());
        }

        if response.code != RESPONSE_CODE_OK {
            let description = response.description.unwrap_or_else(|| "None".to_owned());
            return Err(if let Some(code) = MorpheusdError::from_repr(response.code) {
                RpcError::Morpheusd { code, description }.into()
            } else {
                let code = response.code;
                RpcError::UnknownMorpheusd { code, description }.into()
            });
        }

        trace!("Got response {:?}", response);
        Ok(response)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::repo::RpcProfileRepository;
    use osg::model::ProfileId;
    use osg::repo::{ProfileExplorer, ProfileRepository};
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    #[ignore]
    fn test_server_calls() -> Fallible<()> {
        let addr = "127.0.0.1:6161".parse()?;
        let timeout = Duration::from_secs(5);
        let mut repo = RpcProfileRepository::new(&addr, timeout)?;

        assert_eq!(repo.list_nodes()?.len(), 0);

        let my_id = ProfileId::from_str("IezbeWGSY2dqcUBqT8K7R14xr")?;
        let my_data = ProfileData::new(&my_id);
        repo.set(my_id.clone(), my_data.clone())?;
        let me = repo.get_node(&my_id)?;
        let peer_id = ProfileId::from_str("Iez25N5WZ1Q6TQpgpyYgiu9gTX")?;
        let peer_data = ProfileData::new(&peer_id);
        repo.set(peer_id.clone(), peer_data.clone())?;
        let peer = repo.get_node(&peer_id)?;

        assert_eq!(repo.list_nodes()?.len(), 2);
        // NOTE current set() implementation adds node then sets empty osg attribute
        assert_eq!(me.borrow().version()?, 1);
        assert_eq!(me.borrow().links()?.len(), 0);
        assert_eq!(repo.followers(&my_id)?.len(), 0);
        assert_eq!(peer.borrow().version()?, 1);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(repo.followers(&peer_id)?.len(), 0);

        let link = me.borrow_mut().create_link(&peer_id)?;
        assert_eq!(link.peer_profile, peer_id);
        assert_eq!(repo.list_nodes()?.len(), 2);
        assert_eq!(me.borrow().version()?, 1);
        assert_eq!(me.borrow().links()?.len(), 1);
        assert_eq!(me.borrow().links()?[0].peer_profile, peer_id);
        assert_eq!(repo.followers(&my_id)?.len(), 0);
        assert_eq!(peer.borrow().version()?, 1);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(repo.followers(&peer_id)?[0].peer_profile, my_id);

        me.borrow_mut().remove_link(&peer_id)?;
        assert_eq!(repo.list_nodes()?.len(), 2);
        assert_eq!(me.borrow().version()?, 1);
        assert_eq!(me.borrow().links()?.len(), 0);
        assert_eq!(repo.followers(&my_id)?.len(), 0);
        assert_eq!(peer.borrow().version()?, 1);
        assert_eq!(peer.borrow().links()?.len(), 0);
        assert_eq!(repo.followers(&peer_id)?.len(), 0);

        let attr_id = "1 2 3".to_owned();
        let attr_val = "one two three".to_owned();
        assert_eq!(me.borrow().attributes()?.len(), 0);
        assert_eq!(peer.borrow().attributes()?.len(), 0);
        me.borrow_mut().set_attribute(&attr_id, &attr_val)?;
        assert_eq!(me.borrow().version()?, 1);
        assert_eq!(me.borrow().attributes()?.len(), 1);
        assert_eq!(me.borrow().attributes()?.get(&attr_id), Some(&attr_val));
        assert_eq!(peer.borrow().version()?, 1);
        assert_eq!(peer.borrow().attributes()?.len(), 0);
        me.borrow_mut().clear_attribute(&attr_id)?;
        assert_eq!(me.borrow().version()?, 1);
        assert_eq!(me.borrow().attributes()?.len(), 0);
        assert_eq!(me.borrow().attributes()?.len(), 0);

        me.borrow_mut().set_version(42)?;
        assert_eq!(me.borrow().version()?, 42);

        assert_eq!(repo.list_nodes()?.len(), 2);
        repo.clear(&my_id)?;
        repo.clear(&peer_id)?;
        // NOTE deleting nodes erases all details and keeps an empty profile as a tombstone for followers
        assert_eq!(repo.list_nodes()?.len(), 2);
        // NOTE current clear() implementation deletes node, then adds it back (update tombstone and set empty osg attribute)
        assert_eq!(me.borrow().version()?, 43);
        assert_eq!(peer.borrow().version()?, 2);

        Ok(())
    }
}
