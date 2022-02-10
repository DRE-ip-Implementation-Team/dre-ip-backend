use std::collections::HashMap;
use std::ops::Deref;

use dre_ip::CandidateTotals;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    election::{CandidateID, DreipGroup, Election},
    mongodb::{DbEntity, Id},
};

use super::ballot_core::{Audited, BallotCore, BallotState, Confirmed, Unconfirmed};

/// A ballot from the database, with its unique ID.
/// Also contains an election and question ID foreign key.
#[derive(Clone, Serialize, Deserialize)]
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

    pub fn audit(self) -> Ballot<Audited> {
        Ballot {
            id: self.id,
            election_id: self.election_id,
            question_id: self.question_id,
            ballot: self.ballot.audit(),
        }
    }

    pub fn confirm<'a>(self, totals: impl Into<Option<&'a mut HashMap<CandidateID, &'a mut CandidateTotals<DreipGroup>>>>) -> Ballot<Confirmed> {
        Ballot {
            id: self.id,
            election_id: self.election_id,
            question_id: self.question_id,
            ballot: self.ballot.confirm(totals),
        }
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

/// A ballot that is either Confirmed or Audited.
/// With the untagged representation, `Ballot<Audited>` and
/// `Ballot<Confirmed>` can both directly deserialize to this type.
#[derive(Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FinishedBallot {
    Audited(Ballot<Audited>),
    Confirmed(Ballot<Confirmed>),
}
