use std::time::SystemTime;

use serde_derive::{Deserialize, Serialize};

pub use did::model::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claim {
    id: ContentId,
    pub subject_id: ProfileId,
    pub schema: ContentId,
    pub content: serde_json::Value,
    pub proof: Vec<ClaimProof>,
    pub presentation: Vec<ClaimPresentation>,
}

impl Claim {
    pub fn new(subject_id: ProfileId, schema: impl ToString, content: serde_json::Value) -> Self {
        let mut this = Self {
            id: Default::default(),
            subject_id,
            schema: schema.to_string(),
            content,
            proof: vec![],
            presentation: vec![],
        };
        this.id = this.content_hash();
        this
    }

    fn content_hash(&self) -> ContentId {
        // TODO
        // unimplemented!()
        use rand::{distributions::Alphanumeric, Rng};
        rand::thread_rng().sample_iter(&Alphanumeric).take(16).collect()
    }

    pub fn id(&self) -> ContentId {
        self.id.clone()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, PartialOrd, Serialize)]
pub struct ClaimProof {
    claim_id: String,
    witness: String,   // TODO DID
    signature: String, // TODO multicrypto signature
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ClaimPresentation {
    // TODO: shared_with_subject, usable_for_purpose, expires_at, etc
    journal: Vec<String>, // TODO links to multihash stored on ledger
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
