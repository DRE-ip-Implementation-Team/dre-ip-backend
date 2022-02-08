use std::ops::Deref;

use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    election::{CandidateID, Election},
    mongodb::{DbEntity, Id},
};

use super::ballot_core::{BallotCore, BallotState, FinishedBallot, Unconfirmed};

/// A ballot from the database, with its unique ID.
/// Also contains an election and question ID foreign key.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot<S: BallotState> {
    #[serde(rename = "_id")]
    pub id: Id,
    /// Foreign Key election ID.
    pub election_id: Id,
    /// Foreign Key question ID.
    pub question_id: Id,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore<S>,
}

impl Ballot<Unconfirmed> {
    /// Create a new ballot. Can only fail if there are duplicate candidate IDs passed in.
    pub fn new(election_id: Id, question_id: Id, yes_candidate: CandidateID,
               no_candidates: impl IntoIterator<Item = CandidateID>,
               election: &Election, rng: impl RngCore + CryptoRng) -> Option<Self> {
        let id = Id::new();
        let crypto = election.crypto.create_ballot(rng, id.to_bytes(), yes_candidate, no_candidates)?;
        let ballot = BallotCore {
            crypto,
            state: Unconfirmed,
        };

        Some(Self {
            id,
            election_id,
            question_id,
            ballot,
        })
    }
}

impl<S: BallotState> Deref for Ballot<S> {
    type Target = BallotCore<S>;

    fn deref(&self) -> &Self::Target {
        &self.ballot
    }
}

impl<S: BallotState> DbEntity for Ballot<S> {
    fn id(&self) -> Id {
        self.id
    }
}

/// Marker trait for types that can be considered a database ballot.
pub trait DbBallot {}
impl<S: BallotState> DbBallot for Ballot<S> {}
impl DbBallot for FinishedBallot {}
