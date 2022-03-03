use std::collections::HashMap;
use std::fmt::Debug;

use dre_ip::{Ballot as DreipBallot, CandidateTotals, NoSecrets, VoteSecrets};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::election::{CandidateID, DreipGroup};

pub type BallotCrypto<S> = DreipBallot<CandidateID, DreipGroup, S>;

/// Core ballot data, as stored in the database.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BallotCore<S: BallotState> {
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
            crypto: self.crypto,
            state: Audited,
        }
    }

    /// Confirm this ballot, incrementing the `CandidateTotals` if given.
    pub fn confirm<'a, 'b: 'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateID, &'b mut CandidateTotals<DreipGroup>>>>,
    ) -> BallotCore<Confirmed> {
        BallotCore {
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

pub const UNCONFIRMED: &str = "Unconfirmed";

impl AsRef<[u8]> for Unconfirmed {
    fn as_ref(&self) -> &[u8] {
        UNCONFIRMED.as_bytes()
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

pub const AUDITED: &str = "Audited";

impl AsRef<[u8]> for Audited {
    fn as_ref(&self) -> &[u8] {
        AUDITED.as_bytes()
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

pub const CONFIRMED: &str = "Confirmed";

impl AsRef<[u8]> for Confirmed {
    fn as_ref(&self) -> &[u8] {
        CONFIRMED.as_bytes()
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
