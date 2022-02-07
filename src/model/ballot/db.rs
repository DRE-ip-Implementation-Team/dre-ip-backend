use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::ballot_core::BallotCore;

/// A ballot from the database, with its unique ID.
/// Also contains an election and question ID foreign key.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub ballot: NewBallot,
}

impl Deref for Ballot {
    type Target = BallotCore;

    fn deref(&self) -> &Self::Target {
        &self.ballot.ballot
    }
}

impl DbEntity for Ballot {
    fn id(&self) -> Id {
        self.id
    }
}

/// An ID-less ballot ready for database insertion.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct NewBallot {
    /// Foreign Key election ID.
    pub election_id: Id,
    /// Foreign Key question ID.
    pub question_id: Id,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore,
}
