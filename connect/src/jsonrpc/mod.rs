//use mercury_home_protocol::*;
//use ::DAppPermission;



pub mod server_dispatcher;
pub use self::server_dispatcher::DAppEndpointDispatcherJsonRpc;



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
