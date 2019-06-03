use std::collections::HashMap;

use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

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

// TODO do something better than just compile
impl ProfileAuthJournal for InMemoryProfileAuthJournal {
    fn last_state(&self) -> AsyncFallible<JournalState> {
        Box::new(Ok(JournalState::Transaction(vec![])).into_future())
    }

    fn transactions(
        &self,
        id: &ProfileId,
        until_state: Option<JournalState>,
    ) -> AsyncFallible<Vec<ProfileTransaction>> {
        Box::new(Ok(vec![]).into_future())
    }

    fn get(&self, id: &ProfileId, state: Option<JournalState>) -> AsyncFallible<ProfileAuthData> {
        let mut auth = ProfileAuthData::implicit(id);
        let fut = self.transactions(id, state).map(move |transactions| {
            let ops: Vec<_> = transactions
                .iter()
                .flat_map(|transaction| transaction.ops().iter().cloned())
                .collect();
            auth.apply(&ops);
            auth
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
