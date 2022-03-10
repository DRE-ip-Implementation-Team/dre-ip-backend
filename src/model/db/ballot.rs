use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Utc};
use dre_ip::CandidateTotals;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    election::{CandidateID, DreipGroup},
    mongodb::Id,
};

use crate::model::base::{Audited, BallotCore, BallotState, Confirmed, Unconfirmed};

use super::Election;

/// A ballot from the database, with its unique ID.
/// Also contains an election and question ID foreign key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot<S: BallotState> {
    #[serde(rename = "_id")]
    pub id: Id,
    /// Foreign Key election ID.
    pub election_id: Id,
    /// Foreign Key question ID.
    pub question_id: Id,
    /// Ballot creation time, used to automatically expire unconfirmed votes.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub creation_time: DateTime<Utc>,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore<S>,
}

impl Ballot<Unconfirmed> {
    /// Create a new ballot. Can only fail if there are duplicate candidate IDs passed in.
    pub fn new<S>(
        question_id: Id,
        yes_candidate: CandidateID,
        no_candidates: impl IntoIterator<Item = CandidateID>,
        election: &Election<S>,
        rng: impl RngCore + CryptoRng,
    ) -> Option<Self> {
        let id = Id::new();
        let election_id = election.id;
        let creation_time = Utc::now();
        let crypto =
            election
                .crypto
                .create_ballot(rng, id.to_bytes(), yes_candidate, no_candidates)?;
        let ballot = BallotCore {
            crypto,
            state: Unconfirmed,
        };

        Some(Self {
            id,
            election_id,
            question_id,
            creation_time,
            ballot,
        })
    }

    pub fn audit(self) -> Ballot<Audited> {
        Ballot {
            id: self.id,
            election_id: self.election_id,
            question_id: self.question_id,
            creation_time: self.creation_time,
            ballot: self.ballot.audit(),
        }
    }

    pub fn confirm<'a, 'b: 'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateID, &'b mut CandidateTotals<DreipGroup>>>>,
    ) -> Ballot<Confirmed> {
        Ballot {
            id: self.id,
            election_id: self.election_id,
            question_id: self.question_id,
            creation_time: self.creation_time,
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

impl<S: BallotState> DerefMut for Ballot<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.ballot
    }
}

/// A ballot that is either Confirmed or Audited.
/// With the untagged representation, `Ballot<Audited>` and
/// `Ballot<Confirmed>` can both directly deserialize to this type.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FinishedBallot {
    Audited(Ballot<Audited>),
    Confirmed(Ballot<Confirmed>),
}
