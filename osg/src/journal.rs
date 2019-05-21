use std::collections::HashMap;

use futures::prelude::*;
use serde_derive::{Deserialize, Serialize};

use crate::model::*;

pub trait ProfileAuthJournal {
    fn get(&self, id: &ProfileId) -> AsyncFallible<ProfileAuthData>;
    fn get_ops(&self, id: &ProfileId) -> AsyncFallible<Vec<ProfileTransaction>>;
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
    fn get(&self, id: &ProfileId) -> AsyncFallible<ProfileAuthData> {
        Box::new(Ok(ProfileAuthData::implicit(id)).into_future())
    }

    fn get_ops(&self, id: &ProfileId) -> AsyncFallible<Vec<ProfileTransaction>> {
        Box::new(Ok(vec![]).into_future())
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
