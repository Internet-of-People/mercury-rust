use serde::{Deserialize, Serialize};

use did::model::ProfileId;
use mercury_home_protocol::{AsyncFallible, RelationHalfProof, RelationProof};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct DAppAction(Vec<u8>);

//#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
//pub struct DeviceAuthorization(Vec<u8>);

// User interface (probably implemented with platform-native GUI) for actions
// that are initiated by the SDK and require some kind of user interaction
pub trait UserInteractor {
    // Initialize system components and configuration where user interaction is needed,
    // e.g. HD wallets need manually saving generated new seed or entering old one
    fn initialize(&self) -> AsyncFallible<()>;

    // An action requested by a distributed application needs
    // explicit user confirmation.
    // TODO how to show a human-readable summary of the action (i.e. binary to be signed)
    //      making sure it's not a fake/misinterpreted description?
    fn confirm_dappaction(&self, action: &DAppAction) -> AsyncFallible<()>;

    fn confirm_pairing(&self, request: &RelationHalfProof) -> AsyncFallible<()>;

    fn notify_pairing(&self, response: &RelationProof) -> AsyncFallible<()>;

    // Select a profile to be used by a dApp. It can be either an existing one
    // or the user can create a new one (using a KeyVault) to be selected.
    // TODO this should open something nearly identical to manage_profiles()
    fn select_profile(&self) -> AsyncFallible<ProfileId>;
}
