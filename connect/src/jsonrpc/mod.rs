use std::rc::Rc;

use serde_json;

use mercury_home_protocol::*;
use ::*;



pub mod server_dispatcher;
pub use self::server_dispatcher::StreamingJsonRpc;



#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct JsonRpcRequest
{
    jsonrpc: String,
    id: serde_json::Value,
    method: String,
    params: serde_json::Value,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct JsonRpcResponse
{
    jsonrpc: String,
    id: serde_json::Value,
    result: serde_json::Value,
    error: Option<String>,
}



pub trait JsonRpcMethodDispatcher
{
    fn dispatch(&self, params: serde_json::Value)
        -> AsyncResult<serde_json::Value, serde_json::Value>;
}



pub struct JsonRpcDAppEndpointDispatcher
{
    endpoint: Rc<DAppEndpoint>,
}

impl JsonRpcDAppEndpointDispatcher
{
    pub fn new(endpoint: Rc<DAppEndpoint>) -> Self
        { Self { endpoint } }
}

impl JsonRpcMethodDispatcher for JsonRpcDAppEndpointDispatcher
{
    fn dispatch(&self, params: serde_json::Value)
        -> AsyncResult<serde_json::Value, serde_json::Value>
    {
        // TODO translate Json req -> func params, call func, translate func result -> json response
        unimplemented!()
    }
}

//#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
//pub enum Request
//{
//    DAppSessionRequest(DAppSessionParams),
//}
//
//pub enum Response
//{
//    DAppSessionRequest(String),
//}
//
//
//#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
//pub struct DAppSessionParams
//{
//    dapp: ApplicationId,
//    authorization: Option<DAppPermission>
//}
