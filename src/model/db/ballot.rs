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
    /// Audit this ballot.
    pub fn audit(self) -> BallotCore<Audited> {
        BallotCore {
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
            election_id: self.election_id,
            question_id: self.question_id,
            creation_time: self.creation_time,
            crypto: self.crypto.confirm(totals.into()),
            state: Confirmed,
        }
    }
}

/// A ballot from the database, with its unique ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ballot<S: BallotState> {
    #[serde(rename = "_id")]
    pub id: Id,
    /// Ballot contents.
    #[serde(flatten)]
    pub ballot: BallotCore<S>,
}

impl Ballot<Unconfirmed> {
    /// Create a new ballot. Can only fail if there are duplicate candidate IDs passed in.
    pub fn new(
        question_id: Id,
        yes_candidate: CandidateId,
        no_candidates: impl IntoIterator<Item = CandidateId>,
        election: &Election,
        rng: impl RngCore + CryptoRng,
    ) -> Option<Self> {
        let id = Id::new();
        let election_id = election.id;
        let creation_time = Utc::now();
        let crypto = DreipBallot::new(
            rng,
            election.crypto.g1,
            election.crypto.g2,
            id.to_bytes(),
            yes_candidate,
            no_candidates,
        )?;
        let ballot = BallotCore {
            election_id,
            question_id,
            creation_time,
            crypto,
            state: Unconfirmed,
        };

        Some(Self { id, ballot })
    }

    pub fn audit(self) -> Ballot<Audited> {
        Ballot {
            id: self.id,
            ballot: self.ballot.audit(),
        }
    }

    pub fn confirm<'a, 'b: 'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateId, &'b mut CandidateTotals<DreipGroup>>>>,
    ) -> Ballot<Confirmed> {
        Ballot {
            id: self.id,
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
