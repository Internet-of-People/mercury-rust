use std::collections::HashMap;
use std::time::SystemTime;

use async_trait::async_trait;
use failure::Fallible;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

pub type TransactionId = Vec<u8>;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JournalState {
    TimeStamp(SystemTime), // TODO is this an absolute timestamp or can this be relaxed?
    Transaction(TransactionId),
    Block { height: u64, hash: Vec<u8> },
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

#[async_trait(?Send)]
pub trait ProfileAuthJournal {
    async fn last_state(&self) -> Fallible<JournalState>;
    async fn transactions(
        &self,
        id: &ProfileId,
        until_state: Option<JournalState>,
    ) -> Fallible<Vec<ProfileTransaction>>;

    async fn get(&self, id: &ProfileId, state: Option<JournalState>) -> Fallible<ProfileAuthData>;

    // TODO do we need an explicit Grant here or will it be handled in within the implementation?
    async fn update(&self, operations: &[ProfileAuthOperation]) -> Fallible<ProfileTransaction>;
    async fn remove(&self, id: &ProfileId) -> Fallible<ProfileTransaction>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InMemoryProfileAuthJournal {
    profiles: HashMap<ProfileId, ProfileAuthData>,
}

// TODO do something better than just compile
#[async_trait(?Send)]
impl ProfileAuthJournal for InMemoryProfileAuthJournal {
    async fn last_state(&self) -> Fallible<JournalState> {
        Ok(JournalState::Transaction(vec![]))
    }

    async fn transactions(
        &self,
        _id: &ProfileId,
        _until_state: Option<JournalState>,
    ) -> Fallible<Vec<ProfileTransaction>> {
        Ok(vec![])
    }

    async fn get(&self, id: &ProfileId, state: Option<JournalState>) -> Fallible<ProfileAuthData> {
        let mut auth = ProfileAuthData::implicit(id);
        let transactions = self.transactions(id, state).await?;
        let ops: Vec<_> =
            transactions.iter().flat_map(|transaction| transaction.ops().iter().cloned()).collect();
        auth.apply(&ops)?;
        Ok(auth)
    }

    async fn update(&self, operations: &[ProfileAuthOperation]) -> Fallible<ProfileTransaction> {
        let transaction = ProfileTransaction::new(operations, &[]);
        Ok(transaction)
    }

    async fn remove(&self, id: &ProfileId) -> Fallible<ProfileTransaction> {
        let transaction =
            ProfileTransaction::new(&[ProfileAuthOperation::Remove(id.to_owned())], &[]);
        Ok(transaction)
    }
}
