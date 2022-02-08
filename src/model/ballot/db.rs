use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::ballot_core::{BallotCore, BallotState, FinishedBallot};

/// A ballot from the database, with its unique ID.
/// Also contains an election and question ID foreign key.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot<S: BallotState> {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub ballot: NewBallot<S>,
}

impl<S: BallotState> Deref for Ballot<S> {
    type Target = BallotCore<S>;

    fn deref(&self) -> &Self::Target {
        &self.ballot.ballot
    }
}

impl<S: BallotState> DbEntity for Ballot<S> {
    fn id(&self) -> Id {
        self.id
    }
}

/// An ID-less ballot ready for database insertion.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct NewBallot<S: BallotState> {
    /// Foreign Key election ID.
    pub election_id: Id,
    /// Foreign Key question ID.
    pub question_id: Id,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore<S>,
}

/// Marker trait for types that can be considered a database ballot.
pub trait DbBallot {}
impl<S: BallotState> DbBallot for Ballot<S> {}
impl<S: BallotState> DbBallot for NewBallot<S> {}
impl DbBallot for FinishedBallot {}
