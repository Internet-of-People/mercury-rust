use failure::Fallible;
use serde_derive::{Deserialize, Serialize};

use osg::model::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct Envelope {
    pub(crate) target: String,

    #[serde(with = "serde_bytes")]
    pub(crate) payload: Vec<u8>,
}

impl Envelope {
    pub(crate) fn new(target: &str, payload: Vec<u8>) -> Self {
        Self { target: target.to_owned(), payload }
    }

    pub(crate) fn from<T: serde::Serialize>(target: &str, payload: T) -> Fallible<Self> {
        let payload_bin = rmp_serde::to_vec_named(&payload)?;
        Ok(Self::new(target, payload_bin))
    }
}

pub type MessageId = u64;
pub type ResponseCode = u8;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Request<T> {
    rid: MessageId,
    method: String,
    params: T,
    commit: Option<bool>,
}

impl<T> Request<T>
where
    T: serde::Serialize,
{
    pub(crate) fn new(rid: MessageId, method: &str, params: T) -> Self {
        Self {
            rid,
            method: method.to_owned(),
            params,
            commit: Some(true), // TODO consider how to fill up this field in different requests and repository operations
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Response {
    pub rid: MessageId,
    pub code: ResponseCode,
    pub description: Option<String>,
    pub reply: Option<rmpv::Value>,
}

impl Response {
    pub fn new(
        rid: MessageId,
        code: ResponseCode,
        description: Option<String>,
        reply: Option<rmpv::Value>,
    ) -> Self {
        Self { rid, code, description, reply }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Node {
    pub(crate) id: ProfileId,
}

pub(crate) type AddNodeParams = Node;
pub(crate) type RemoveNodeParams = Node;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SetNodeAttributeParams {
    pub(crate) id: ProfileId,
    pub(crate) key: AttributeId,
    #[serde(with = "serde_bytes")]
    pub(crate) value: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct NodeAttribute {
    pub(crate) id: ProfileId,
    pub(crate) key: AttributeId,
}

pub(crate) type GetNodeAttributeParams = NodeAttribute;
pub(crate) type ClearNodeAttributeParams = NodeAttribute;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct ListNodesParams {}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct Edge {
    pub(crate) source: ProfileId,
    pub(crate) target: ProfileId,
}

pub(crate) type ListInEdgesParams = Node;
pub(crate) type ListInEdgesReply = Vec<ProfileId>;

pub(crate) type ListOutEdgesParams = Node;
pub(crate) type ListOutEdgesReply = Vec<ProfileId>;

pub(crate) type AddEdgeParams = Edge;
pub(crate) type RemoveEdgeParams = Edge;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct SetEdgeAttributeParams {
    pub(crate) source: ProfileId,
    pub(crate) target: ProfileId,
    pub(crate) key: AttributeId,
    #[serde(with = "serde_bytes")]
    pub(crate) value: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub(crate) struct EdgeAttribute {
    pub(crate) source: ProfileId,
    pub(crate) target: ProfileId,
    pub(crate) key: AttributeId,
}

pub(crate) type GetEdgeAttributeParams = EdgeAttribute;
pub(crate) type ClearEdgeAttributeParams = EdgeAttribute;

#[test]
fn test_serialization_concept() {
    let original_envelope = {
        let params = AddEdgeParams {
            source: "Iez21JXEtMzXjbCK6BAYFU9ewX".parse::<ProfileId>().unwrap(),
            target: "IezpmXKKc2QRZpXbzGV62MgKe".parse::<ProfileId>().unwrap(),
        };
        let request = Request::new(1, "add_edge", params);
        // println!("request: {:#?}", request);
        Envelope::from("target", request).expect("Failed to build envelope from request")
    };

    // println!("envelope: {:?}", original_envelope);
    let envelope_bytes =
        rmp_serde::encode::to_vec_named(&original_envelope).expect("Failed to serialize envelope");

    let read_envelope: Envelope =
        rmp_serde::decode::from_slice(&envelope_bytes).expect("Failed to parse envelope");
    assert_eq!(read_envelope, original_envelope);
    // println!("envelope: {:?}", read_envelope);
}
