pub mod server_dispatcher;
pub use self::server_dispatcher::DAppSessionDispatcherJsonRpc;


#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct EchoParams
{
    pub message: String,
}

