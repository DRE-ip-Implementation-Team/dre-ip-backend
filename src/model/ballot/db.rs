use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::ballot_core::BallotCore;

/// A ballot from the database, with its unique ID.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    ballot: BallotCore,
}

impl Deref for Ballot {
    type Target = BallotCore;

    fn deref(&self) -> &Self::Target {
        &self.ballot
    }
}

impl DbEntity for Ballot {
    fn id(&self) -> Id {
        self.id
    }
}
