use std::time::SystemTime;

use serde_derive::{Deserialize, Serialize};

pub use did::model::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Claim {
    id: String,     // TODO multihash?
    schema: String, // TODO multihash?
    content: serde_json::Value,
    proof: Vec<ClaimProof>,
    presentation: Vec<ClaimPresentation>,
}

impl Claim {
    pub fn new(id: &str, schema: &str, content: serde_json::Value) -> Self {
        Self {
            id: id.to_owned(),
            schema: schema.to_owned(),
            content,
            proof: vec![],
            presentation: vec![],
        }
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
