use std::collections::HashMap;
use std::time::SystemTime;

use serde_derive::{Deserialize, Serialize};

use crate::claim_schema::SchemaId;
pub use did::model::*;

// TODO this overlaps with JournalState, maybe they could be merged
pub type Version = u64; // monotonically increasing, e.g. normal version, unix datetime or blockheight
pub type AttributeId = String;
pub type AttributeValue = String;
pub type AttributeMap = HashMap<AttributeId, AttributeValue>;
pub type ClaimId = ContentId;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct ClaimProof {
    claim_id: String,
    witness: String,   // TODO DID
    signature: String, // TODO multicrypto signature
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ClaimPresentation {
    // TODO: shared_with_subject, usable_for_purpose, expires_at, etc
    journal: Vec<String>, // TODO links to multihash stored on ledger
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Claim {
    id: ClaimId,
    pub subject_id: ProfileId,
    pub schema: SchemaId,
    pub content: Vec<u8>,
    pub proof: Vec<ClaimProof>,
    pub presentation: Vec<ClaimPresentation>,
}

impl Claim {
    pub fn new(subject_id: ProfileId, schema: impl ToString, content: Vec<u8>) -> Self {
        let mut this = Self {
            id: Default::default(),
            subject_id,
            schema: schema.to_string(),
            content,
            proof: vec![],
            presentation: vec![],
        };
        this.id = HashableClaimContent::from(&this).content_hash();
        this
    }

    pub fn id(&self) -> ClaimId {
        self.id.clone()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct HashableClaimContent {
    pub subject_id: ProfileId,
    pub schema: SchemaId,
    pub content: Vec<u8>,
    pub proof: Vec<ClaimProof>,
    pub presentation: Vec<ClaimPresentation>,
}

impl HashableClaimContent {
    fn content_hash(&self) -> ContentId {
        // TODO make it better usable from other languages
        let content_res = serde_json::to_vec(self);

        // NOTE docs: "Serialization can fail if `T`'s implementation of `Serialize` decides to
        //      fail, or if `T` contains a map with non-string keys." This cannot happen here.
        let content = content_res.unwrap();

        // NOTE docs: "Will return an error if the specified hash type is not supported", cannot happen.
        let hash = multihash::encode(multihash::Hash::Keccak256, &content).unwrap();
        multibase::encode(multibase::Base64, &hash)
    }
}

impl From<&Claim> for HashableClaimContent {
    fn from(src: &Claim) -> Self {
        Self {
            subject_id: src.subject_id.to_owned(),
            schema: src.schema.to_owned(),
            content: src.content.to_owned(),
            proof: src.proof.to_owned(),
            presentation: src.presentation.to_owned(),
        }
    }
}

// TODO generalize links (i.e. edges) between two profiles into verifiable claims,
//      i.e. signed hyperedges in the graph with any number of referenced profiles
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct Link {
    pub peer_profile: ProfileId,
    // pub id: LinkId,
    // pub source_profile: ProfileId, // NOTE this might be needed when serialized, but redundant when in-memory
    // pub metadata: HashMap<AttributeId,AttributeValue>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PublicProfileData {
    public_key: PublicKey,
    version: Version,
    attributes: AttributeMap,
    // TODO remove this, links/contacts should be a special case of claims and filtered from them
    links: Vec<Link>,
    // TODO consider adding a signature of the profile data here
}

impl PublicProfileData {
    pub fn new(
        public_key: PublicKey,
        version: Version,
        links: Vec<Link>,
        attributes: AttributeMap,
    ) -> Self {
        Self { public_key, version, links, attributes }
    }

    pub fn empty(public_key: &PublicKey) -> Self {
        Self::new(public_key.to_owned(), 1, Default::default(), Default::default())
    }

    pub fn tombstone(public_key: &PublicKey, last_version: Version) -> Self {
        Self {
            public_key: public_key.to_owned(),
            version: last_version + 1,
            links: Default::default(),
            attributes: Default::default(),
        }
    }

    pub fn id(&self) -> ProfileId {
        use keyvault::PublicKey as KeyVaultPublicKey;
        self.public_key.key_id()
    }

    pub fn public_key(&self) -> PublicKey {
        self.public_key.clone() // TODO in the dev branches this is already Copy, remove cloning after it's merged
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn increase_version(&mut self) {
        self.version += 1;
    }
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    pub fn links(&self) -> &Vec<Link> {
        &self.links
    }

    pub fn create_link(&mut self, with_id: &ProfileId) -> Link {
        let link = Link { peer_profile: with_id.to_owned() };
        if !self.links.contains(&link) {
            self.links.push(link.clone());
        }
        link
    }

    pub fn remove_link(&mut self, with_id: &ProfileId) {
        self.links.retain(|link| link.peer_profile != *with_id)
    }

    pub fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    pub fn set_attribute(&mut self, key: AttributeId, value: AttributeValue) {
        self.attributes.insert(key, value);
    }

    pub fn clear_attribute(&mut self, key: &AttributeId) {
        self.attributes.remove(key);
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PrivateProfileData {
    public_data: PublicProfileData,
    claims: Vec<Claim>,
    private_data: Vec<u8>,
}

impl PrivateProfileData {
    pub fn new(public_data: PublicProfileData, private_data: Vec<u8>, claims: Vec<Claim>) -> Self {
        Self { public_data, private_data, claims }
    }

    // TODO The lower-level Mercury network layer should not be aware of claims and the
    //      structure of any private encrypted data in general. It should operate with a different data type
    //      after this is splitted into a lower-level part of Mercury
    #[deprecated]
    pub fn without_morpheus_claims(public_data: PublicProfileData, private_data: Vec<u8>) -> Self {
        Self::new(public_data, private_data, vec![])
    }

    pub fn from_public(public_data: PublicProfileData) -> Self {
        Self::new(public_data, vec![], vec![])
    }

    pub fn empty(public_key: &PublicKey) -> Self {
        Self::from_public(PublicProfileData::empty(public_key))
    }

    pub fn tombstone(public_key: &PublicKey, last_version: Version) -> Self {
        Self {
            public_data: PublicProfileData::tombstone(public_key, last_version),
            claims: Default::default(),
            private_data: Default::default(),
        }
    }

    pub fn public_data(&self) -> PublicProfileData {
        self.public_data.clone()
    }
    pub fn claims(&self) -> Vec<Claim> {
        self.claims.clone()
    }
    pub fn private_data(&self) -> Vec<u8> {
        self.private_data.clone()
    }

    pub fn mut_public_data(&mut self) -> &mut PublicProfileData {
        &mut self.public_data
    }
    pub fn mut_claims(&mut self) -> &mut Vec<Claim> {
        &mut self.claims
    }
    pub fn mut_private_data(&mut self) -> &mut Vec<u8> {
        &mut self.private_data
    }

    pub fn id(&self) -> ProfileId {
        self.public_data.id()
    }
    pub fn version(&self) -> Version {
        self.public_data.version()
    }
    pub fn public_key(&self) -> PublicKey {
        self.public_data.public_key()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Grant {
    Impersonate,
    Restore,
    Modify,
    Support,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileGrant {
    key_id: KeyId,
    grant: Grant,
}

impl ProfileGrant {
    pub fn new(key_id: KeyId, grant: Grant) -> Self {
        Self { key_id, grant }
    }
}

// a.k.a DID Document
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileAuthData {
    id: ProfileId,
    timestamp: SystemTime, // TODO is this an absolute timestamp or can this be relaxed?
    grants: Vec<ProfileGrant>,
    services: Vec<String>, // TODO what storage pointers type to use here? Ideally would be multistorage link.
}

impl ProfileAuthData {
    pub fn implicit(key_id: &KeyId) -> Self {
        let id = key_id.to_owned();
        Self {
            id: id.clone(),
            timestamp: SystemTime::now(),
            grants: vec![ProfileGrant { key_id: id, grant: Grant::Impersonate }],
            services: vec![],
        }
    }

    pub fn keys_with_grant(&self, grant: Grant) -> Vec<KeyId> {
        self.grants
            .iter()
            .filter_map(|pg| if pg.grant == grant { Some(pg.key_id.to_owned()) } else { None })
            .collect()
    }

    pub fn grants_of_key(&self, key_id: &KeyId) -> Vec<Grant> {
        self.grants
            .iter()
            .filter_map(|pg| if pg.key_id == *key_id { Some(pg.grant) } else { None })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
