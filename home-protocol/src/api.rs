use std::str;

use crate::*;
use keyvault::PublicKey as KeyVaultPublicKey;

pub const CHANNEL_CAPACITY: usize = 1;

/// Represents a connection to another Profile (Home <-> Persona), (Persona <-> Persona)
#[derive(Clone)]
pub struct PeerContext {
    my_signer: Rc<Signer>,
    peer_pubkey: PublicKey,
}

impl PeerContext {
    pub fn new(my_signer: Rc<Signer>, peer_pubkey: PublicKey) -> Self {
        Self { my_signer, peer_pubkey }
    }

    pub fn my_signer(&self) -> &Signer {
        &*self.my_signer
    }
    pub fn peer_pubkey(&self) -> PublicKey {
        self.peer_pubkey.clone()
    }
    pub fn peer_id(&self) -> ProfileId {
        self.peer_pubkey.key_id()
    }

    pub fn validate(&self, validator: &Validator) -> Result<(), Error> {
        validator.validate_profile_auth(&self.peer_pubkey(), &self.peer_id()).and_then(|valid| {
            if valid {
                Ok(())
            } else {
                Err(ErrorKind::ProfileValidationFailed)?
            }
        })
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
#[derive(Debug)]
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
pub trait Home: ProfileExplorer {
    // NOTE because we support multihash, the id cannot be guessed from the public key
    fn claim(&self, profile: ProfileId) -> AsyncResult<OwnProfile, Error>;

    // TODO this should return only the signed RelationProof of the home hosting the profile
    //      because in this form the home can return malicious changes in the profile
    fn register(
        &self,
        own_prof: OwnProfile,
        half_proof: RelationHalfProof,
        // invite: Option<HomeInvitation>,
    ) -> AsyncResult<OwnProfile, (OwnProfile, Error)>;

    /// By calling this method, any active session of the same profile is closed.
    fn login(&self, proof_of_home: &RelationProof) -> AsyncResult<Rc<HomeSession>, Error>;

    /// The peer in `half_proof` must be hosted on this home server.
    /// Returns Error if the peer is not hosted on this home server or an empty result if it is.
    /// Note that the peer will directly invoke `pair_response` on the initiator's home server and call pair_response to send PairingResponse event
    fn pair_request(&self, half_proof: RelationHalfProof) -> AsyncResult<(), Error>;

    fn pair_response(&self, rel: RelationProof) -> AsyncResult<(), Error>;

    // NOTE initiating a real P2P connection (vs a single frame push notification),
    //      the caller must fill in some message channel to itself.
    //      A successful call returns a channel to callee.
    fn call(
        &self,
        app: ApplicationId,
        call_req: CallRequestDetails,
    ) -> AsyncResult<Option<AppMsgSink>, Error>;

    // TODO consider how to do this in a later milestone
    //    fn presence(&self, rel: Relation, app: ApplicationId) ->
    //        AsyncResult<Option<AppMessageFrame>, Error>;
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

pub trait HomeSession {
    fn update(&self, own_prof: OwnProfile) -> AsyncResult<(), Error>;

    // NOTE newhome is a profile that contains at least one HomeFacet different than this home
    // TODO should we return a modified OwnProfile here with this home removed from the homes of persona facet in profile?
    fn unregister(&self, newhome: Option<Profile>) -> AsyncResult<(), Error>;

    fn events(&self) -> AsyncStream<ProfileEvent, String>;

    // TODO some kind of proof might be needed that the AppId given really belongs to the caller
    // TODO add argument in a later milestone, presence: Option<AppMessageFrame>) ->
    fn checkin_app(&self, app: &ApplicationId) -> AsyncStream<Box<IncomingCall>, String>;

    // TODO remove this after testing
    fn ping(&self, txt: &str) -> AsyncResult<String, Error>;

    // TODO ban features are delayed to a later milestone
    //    fn banned_profiles(&self) -> AsyncResult<Vec<ProfileId>, Error>;
    //    fn ban(&self, profile: &ProfileId) -> AsyncResult<(), Error>;
    //    fn unban(&self, profile: &ProfileId) -> AsyncResult<(), Error>;
}
