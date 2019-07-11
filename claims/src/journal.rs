use std::collections::HashMap;
use std::time::SystemTime;

use futures::prelude::*;
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

pub trait ProfileAuthJournal {
    fn last_state(&self) -> AsyncFallible<JournalState>;
    fn transactions(
        &self,
        id: &ProfileId,
        until_state: Option<JournalState>,
    ) -> AsyncFallible<Vec<ProfileTransaction>>;

    fn get(&self, id: &ProfileId, state: Option<JournalState>) -> AsyncFallible<ProfileAuthData>;

    // TODO do we need an explicit Grant here or will it be handled in within the implementation?
    fn update(&self, operations: &[ProfileAuthOperation]) -> AsyncFallible<ProfileTransaction>;
    fn remove(&self, id: &ProfileId) -> AsyncFallible<ProfileTransaction>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InMemoryProfileAuthJournal {
    profiles: HashMap<ProfileId, ProfileAuthData>,
}

impl InMemoryProfileAuthJournal {
    pub fn apply(_auth: &ProfileAuthData, _ops: &[ProfileAuthOperation]) -> ProfileAuthData {
        unimplemented!()
    }
}

// TODO do something better than just compile
impl ProfileAuthJournal for InMemoryProfileAuthJournal {
    fn last_state(&self) -> AsyncFallible<JournalState> {
        Box::new(Ok(JournalState::Transaction(vec![])).into_future())
    }

    fn transactions(
        &self,
        _id: &ProfileId,
        _until_state: Option<JournalState>,
    ) -> AsyncFallible<Vec<ProfileTransaction>> {
        Box::new(Ok(vec![]).into_future())
    }

    fn get(&self, id: &ProfileId, state: Option<JournalState>) -> AsyncFallible<ProfileAuthData> {
        let auth = ProfileAuthData::implicit(id);
        let fut = self.transactions(id, state).map(move |transactions| {
            let ops: Vec<_> = transactions
                .iter()
                .flat_map(|transaction| transaction.ops().iter().cloned())
                .collect();
            Self::apply(&auth, &ops)
        });

        Box::new(fut)
    }

    fn update(&self, operations: &[ProfileAuthOperation]) -> AsyncFallible<ProfileTransaction> {
        let transaction = ProfileTransaction::new(operations, &[]);
        Box::new(Ok(transaction).into_future())
    }

    fn remove(&self, id: &ProfileId) -> AsyncFallible<ProfileTransaction> {
        let transaction =
            ProfileTransaction::new(&[ProfileAuthOperation::Remove(id.to_owned())], &[]);
        Box::new(Ok(transaction).into_future())
    }
}
