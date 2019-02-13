use serde_derive::{Deserialize, Serialize};

//pub type ProfileId = Vec<u8>;
pub type AttributeId = String;
pub type AttributeValue = String; // StructOpt needs FromStr, MessagePack needs Vec<u8>
pub type Signature = Vec<u8>;
pub type PublicKey = Vec<u8>;
pub type ProfileId = morpheus_keyvault::ed25519::KeyId;

//pub type LinkId = ProfileId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}
