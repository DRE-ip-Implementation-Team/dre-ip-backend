use std::collections::HashMap;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use dre_ip::{Ballot as DreipBallot, CandidateTotals, NoSecrets, VoteSecrets};
use mongodb::bson::to_bson;
use mongodb::bson::{serde_helpers::chrono_datetime_as_bson_datetime, Bson};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::{
    common::election::{CandidateId, DreipGroup},
    mongodb::Id,
};

pub type BallotCrypto<S> = DreipBallot<CandidateId, DreipGroup, S>;

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

/// Trait for the ballot state, enforcing on the type level that secrets are present
/// if and only if the ballot is unconfirmed or audited.
pub trait BallotState: Copy + AsRef<[u8]> {
    /// Do we store the secrets internally?
    type InternalSecrets: Serialize + DeserializeOwned + Debug + Clone;
    /// Do we reveal the secrets in the receipt?
    type ExposedSecrets: Serialize + DeserializeOwned + Debug + Clone;
    /// Convert internal representation into receipt representation.
    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets>;
}

/// Marker type for unconfirmed ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Unconfirmed;

const UNCONFIRMED: &str = "Unconfirmed";

impl AsRef<[u8]> for Unconfirmed {
    fn as_ref(&self) -> &[u8] {
        UNCONFIRMED.as_bytes()
    }
}

impl From<Unconfirmed> for Bson {
    fn from(state: Unconfirmed) -> Self {
        to_bson(&state).expect("Serialisation is infallible")
    }
}

/// Unconfirmed ballots have secrets internally but do not reveal them in receipts.
impl BallotState for Unconfirmed {
    type InternalSecrets = VoteSecrets<DreipGroup>;
    type ExposedSecrets = NoSecrets;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal.confirm(None)
    }
}

/// Marker type for audited ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Audited;

const AUDITED: &str = "Audited";

impl AsRef<[u8]> for Audited {
    fn as_ref(&self) -> &[u8] {
        AUDITED.as_bytes()
    }
}

impl From<Audited> for Bson {
    fn from(state: Audited) -> Self {
        to_bson(&state).expect("Serialisation is infallible")
    }
}

/// Audited ballots have secrets internally and also make them public in receipts.
impl BallotState for Audited {
    type InternalSecrets = VoteSecrets<DreipGroup>;
    type ExposedSecrets = VoteSecrets<DreipGroup>;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal
    }
}

/// Marker type for confirmed ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Confirmed;

const CONFIRMED: &str = "Confirmed";

impl AsRef<[u8]> for Confirmed {
    fn as_ref(&self) -> &[u8] {
        CONFIRMED.as_bytes()
    }
}

impl From<Confirmed> for Bson {
    fn from(state: Confirmed) -> Self {
        to_bson(&state).expect("Serialisation is infallible")
    }
}

/// Confirmed ballots have secrets erased; they are not present internally or in receipts.
impl BallotState for Confirmed {
    type InternalSecrets = NoSecrets;
    type ExposedSecrets = NoSecrets;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal
    }
}
