use mercury_home_protocol::*;
use ::*;



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct GetSessionRequest {
    pub application_id: ApplicationId,
    pub permissions: Option<DAppPermission>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct GetSessionResponse {
    pub profile_id: String,
}



#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct EventNotification {
    pub kind: String,
}

