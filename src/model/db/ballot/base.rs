use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dre_ip::CandidateTotals;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

use crate::model::{
    common::{
        ballot::{Audited, BallotCrypto, BallotState, Confirmed, Unconfirmed},
        election::{CandidateId, DreipGroup},
    },
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
