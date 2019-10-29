use std::str;

use async_trait::async_trait;
use failure::Fallible;

use crate::*;
use keyvault::PublicKey as KeyVaultPublicKey;

pub const CHANNEL_CAPACITY: usize = 1;

/// Represents a connection to another Profile (Home <-> Persona), (Persona <-> Persona)
#[derive(Clone)]
pub struct PeerContext {
    my_signer: Rc<dyn Signer>,
    peer_pubkey: PublicKey,
}

impl PeerContext {
    pub fn new(my_signer: Rc<dyn Signer>, peer_pubkey: PublicKey) -> Self {
        Self { my_signer, peer_pubkey }
    }

    pub fn my_signer(&self) -> &dyn Signer {
        self.my_signer.as_ref()
    }
    pub fn peer_pubkey(&self) -> PublicKey {
        self.peer_pubkey.clone()
    }
    pub fn peer_id(&self) -> ProfileId {
        self.peer_pubkey.key_id()
    }

    pub fn validate(&self, validator: &dyn Validator) -> Result<(), Error> {
        validator.validate_profile_auth(&self.peer_pubkey(), &self.peer_id())
    }
}

pub type AsyncStream<Elem, RemoteErr> = mpsc::Receiver<std::result::Result<Elem, RemoteErr>>;
pub type AsyncSink<Elem, RemoteErr> = mpsc::Sender<std::result::Result<Elem, RemoteErr>>;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct AppMessageFrame(pub Vec<u8>);

pub type AppMsgStream = AsyncStream<AppMessageFrame, String>;
pub type AppMsgSink = AsyncSink<AppMessageFrame, String>;

/// A struct that is passed from the caller to the callee. The callee can examine this
/// before answering the call.
#[derive(Clone, Debug)]
pub struct CallRequestDetails {
    /// Proof for the home server that the caller is authorized to call the callee.
    /// The callee can find out who's calling by looking at `relation`.
    pub relation: RelationProof,

    /// A message that the callee can examine before answering or rejecting a call. Note that the caller is already
    /// known to the callee through `relation`.
    pub init_payload: AppMessageFrame,

    /// The sink half of a channel that routes `AppMessageFrame`s back to the caller. If the caller
    /// does not want to receive any response messages from the callee, `to_caller` should be set to `None`.
    pub to_caller: Option<AppMsgSink>,
}

// Interface to a single home server.
// NOTE authentication is already done when the connection is built,
//      authenticated profile info is available from the connection context
// TODO Home should be derived from DistributedPublicProfileRepository instead on the long run
#[async_trait(?Send)]
pub trait Home: ProfileExplorer {
    // NOTE because we support multihash, the id cannot be guessed from the public key
    async fn claim(&self, profile_id: ProfileId) -> Fallible<RelationProof>;

    // TODO this should return only the signed RelationProof of the home hosting the profile
    //      because in this form the home can return malicious changes in the profile
    async fn register(
        &self,
        half_proof: &RelationHalfProof,
        // invite: Option<HomeInvitation>,
    ) -> Fallible<RelationProof>;

    /// By calling this method, any active session of the same profile is closed.
    async fn login(&self, hosting_proof: &RelationProof) -> Fallible<Rc<dyn HomeSession>>;

    /// The peer in `half_proof` must be hosted on this home server.
    /// Returns Error if the peer is not hosted on this home server or an empty result if it is.
    /// Note that the peer will directly invoke `pair_response` on the initiator's home server and call pair_response to send PairingResponse event
    async fn pair_request(&self, half_proof: &RelationHalfProof) -> Fallible<()>;

    async fn pair_response(&self, relation_proof: &RelationProof) -> Fallible<()>;

    // NOTE initiating a real P2P connection (vs a single frame push notification),
    //      the caller must fill in some message channel to itself.
    //      A successful call returns a channel to callee.
    async fn call(
        &self,
        app: ApplicationId,
        call_req: &CallRequestDetails,
    ) -> Fallible<Option<AppMsgSink>>;

    // TODO consider how to do this in a later milestone
    //    async fn presence(&self, rel: Relation, app: ApplicationId) ->
    //        Fallible<Option<AppMessageFrame>>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProfileEvent {
    Unknown(Vec<u8>), // forward compatibility for protocol extension
    PairingRequest(RelationHalfProof),
    // TODO do we want to distinguish "rejected" and "notYetApproved" states for pairing, i.e. need an explicit rejected response?
    PairingResponse(RelationProof),
    // TODO are these events needed? What others?
    //    HomeBroadcast,
    //    HomeHostingExpiry,
    //    ProfileUpdated, // from a different client instance/session
}

pub trait IncomingCall {
    /// Get a reference to details of the call.
    /// It contains information about the caller party (`relation`), an initial message (`initial_payload`)
    /// If the caller wishes to receive App messages from the calee, a sink should be passed in `to_caller`.
    fn request_details(&self) -> &CallRequestDetails;

    // NOTE this assumes boxed trait objects, if Rc of something else is needed, this must be revised
    // TODO consider offering the possibility to somehow send back a single AppMessageFrame
    //      as a reply to init_payload without a to_callee sink,
    //      either included into this function or an additional method
    /// Indicate to the caller that the call was answered.
    /// If the callee wishes to receive messages from the caller, it has to create a channel
    /// and pass the created sink to `answer()`, which is returned by `call()` on the caller side.
    fn answer(self: Box<Self>, to_callee: Option<AppMsgSink>) -> CallRequestDetails;
}

#[async_trait(?Send)]
pub trait HomeSession {
    async fn backup(&self, own_profile: OwnProfile) -> Fallible<()>;
    async fn restore(&self) -> Fallible<OwnProfile>;

    // NOTE new_home is a profile that contains at least one HomeFacet different than this home
    async fn unregister(&self, new_home: Option<Profile>) -> Fallible<()>;

    fn events(&self) -> AsyncStream<ProfileEvent, String>;

    // TODO some kind of proof might be needed that the AppId given really belongs to the caller
    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> AsyncStream<Box<dyn IncomingCall>, String>;

    // TODO remove this after testing
    async fn ping(&self, txt: &str) -> Fallible<String>;

    // TODO ban features are delayed to a later milestone
    //    async fn banned_profiles(&self) -> Fallible<Vec<ProfileId>>;
    //    async fn ban(&self, profile: &ProfileId) -> Fallible<()>;
    //    async fn unban(&self, profile: &ProfileId) -> Fallible<()>;
}
