use std::time::SystemTime;

use serde_derive::{Deserialize, Serialize};

pub use did::model::*;

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

    pub fn apply(&mut self, _ops: &[ProfileAuthOperation]) {
        unimplemented!()
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

pub type TransactionId = Vec<u8>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JournalState {
    TimeStamp(SystemTime), // TODO is this an absolute timestamp or can this be relaxed?
    Transaction(TransactionId),
    Block { height: u64, hash: Vec<u8> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ProfileAuthOperation {
    Grant(ProfileGrant),
    Revoke(ProfileGrant),
    Remove(ProfileId),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProfileTransaction {
    // new_state: ProfileAuthData, // NOTE it's harder to validate state diffs than to add explicit operations
    operations: Vec<ProfileAuthOperation>,
    succeeds_predecessors: Vec<JournalState>,
}

impl ProfileTransaction {
    pub fn new(ops: &[ProfileAuthOperation], succeeds_predecessors: &[JournalState]) -> Self {
        Self { operations: ops.to_owned(), succeeds_predecessors: succeeds_predecessors.to_owned() }
    }

    pub fn ops(&self) -> &[ProfileAuthOperation] {
        &self.operations
    }

    pub fn predecessors(&self) -> &[JournalState] {
        &self.succeeds_predecessors
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
