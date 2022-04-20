use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use chrono::{DateTime, Utc};
use dre_ip::{Ballot as DreipBallot, CandidateTotals};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    common::{
        ballot::{Audited, BallotCrypto, BallotState, Confirmed, Unconfirmed},
        election::{CandidateId, DreipGroup},
    },
    db::election::Election,
    mongodb::Id,
};

/// Core ballot data, as stored in the database.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BallotCore<S: BallotState> {
    /// Ballot ID. Unlike most IDs, this is an incrementing index, as it will
    /// be directly seen by voters and needs to be user-friendly.
    pub ballot_id: u64,
    /// Foreign Key election ID.
    pub election_id: Id,
    /// Foreign Key question ID.
    pub question_id: Id,
    /// Ballot creation time, used to automatically expire unconfirmed votes.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub creation_time: DateTime<Utc>,
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: BallotCrypto<S::InternalSecrets>,
    /// The current state of the ballot.
    pub state: S,
}

impl BallotCore<Unconfirmed> {
    /// Create a new ballot. Can only fail if there are duplicate candidate IDs passed in.
    pub fn new(
        ballot_id: u64,
        question_id: Id,
        yes_candidate: CandidateId,
        no_candidates: impl IntoIterator<Item = CandidateId>,
        election: &Election,
        rng: impl RngCore + CryptoRng,
    ) -> Option<Self> {
        let election_id = election.id;
        let creation_time = Utc::now();
        let crypto = DreipBallot::new(
            rng,
            election.crypto.g1,
            election.crypto.g2,
            ballot_id.to_le_bytes(),
            yes_candidate,
            no_candidates,
        )?;
        Some(BallotCore {
            ballot_id,
            election_id,
            question_id,
            creation_time,
            crypto,
            state: Unconfirmed,
        })
    }

    /// Audit this ballot.
    pub fn audit(self) -> BallotCore<Audited> {
        BallotCore {
            ballot_id: self.ballot_id,
            election_id: self.election_id,
            question_id: self.question_id,
            creation_time: self.creation_time,
            crypto: self.crypto,
            state: Audited,
        }
    }

    /// Confirm this ballot, incrementing the `CandidateTotals` if given.
    pub fn confirm<'a, 'b: 'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateId, &'b mut CandidateTotals<DreipGroup>>>>,
    ) -> BallotCore<Confirmed> {
        BallotCore {
            ballot_id: self.ballot_id,
            election_id: self.election_id,
            question_id: self.question_id,
            creation_time: self.creation_time,
            crypto: self.crypto.confirm(totals.into()),
            state: Confirmed,
        }
    }
}

/// A newly-created ballot that hasn't made it to the database yet.
pub type NewBallot = BallotCore<Unconfirmed>;

/// A ballot from the database, with its globally unique ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot<S: BallotState> {
    /// This is *NOT* the ballot ID as far as the voting protocol is concerned.
    /// This is just a unique identifier because the database demands one.
    #[serde(rename = "_id")]
    pub internal_id: Id,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore<S>,
}

impl Ballot<Unconfirmed> {
    pub fn audit(self) -> Ballot<Audited> {
        Ballot {
            internal_id: self.internal_id,
            ballot: self.ballot.audit(),
        }
    }

    pub fn confirm<'a, 'b: 'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateId, &'b mut CandidateTotals<DreipGroup>>>>,
    ) -> Ballot<Confirmed> {
        Ballot {
            internal_id: self.internal_id,
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
