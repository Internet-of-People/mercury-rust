use async_trait::async_trait;
use failure::Fallible;
use serde::{Deserialize, Serialize};

use did::model::ProfileId;
use mercury_home_protocol::{RelationHalfProof, RelationProof};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

//#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
//pub struct DeviceAuthorization(Vec<u8>);

// User interface (probably implemented with platform-native GUI) for actions
// that are initiated by the SDK and require some kind of user interaction
#[async_trait]
pub trait UserInteractor {
    // Initialize system components and configuration where user interaction is needed,
    // e.g. HD wallets need manually saving generated new seed or entering old one
    async fn initialize(&self) -> Fallible<()>;

    // An action requested by a distributed application needs
    // explicit user confirmation.
    // TODO how to show a human-readable summary of the action (i.e. binary to be signed)
    //      making sure it's not a fake/misinterpreted description?
    async fn confirm_dappaction(&self, action: &DAppAction) -> Fallible<()>;

    async fn confirm_pairing(&self, request: &RelationHalfProof) -> Fallible<()>;

    async fn notify_pairing(&self, response: &RelationProof) -> Fallible<()>;

    // Select a profile to be used by a dApp. It can be either an existing one
    // or the user can create a new one (using a KeyVault) to be selected.
    // TODO this should open something nearly identical to manage_profiles()
    async fn select_profile(&self) -> Fallible<ProfileId>;
}
