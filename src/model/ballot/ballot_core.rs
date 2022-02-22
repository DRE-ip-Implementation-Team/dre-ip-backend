use std::collections::HashMap;

use dre_ip::{Ballot as DreipBallot, CandidateTotals, Vote as DreipVote};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::election::{CandidateID, DreipGroup};

pub type BallotCrypto<V> = DreipBallot<CandidateID, DreipGroup, V>;

/// Core ballot data, as stored in the database.
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
pub struct BallotCore<S: BallotState> {
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: BallotCrypto<S::InternalVote>,
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
    pub fn confirm<'a>(
        self,
        totals: impl Into<Option<&'a mut HashMap<CandidateID, &'a mut CandidateTotals<DreipGroup>>>>,
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
    /// Is this state represented internally by a `ConfirmedVote` or an `UnconfirmedVote`?
    type InternalVote: DreipVote<DreipGroup> + Serialize + DeserializeOwned + Clone;
    /// Do we show the voter a `ConfirmedVote` or an `UnconfirmedVote` (do we reveal the
    /// secrets in the receipt)?
    type ReceiptVote: DreipVote<DreipGroup> + Serialize + DeserializeOwned + Clone;
    /// Convert internal representation into receipt representation.
    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalVote>,
    ) -> BallotCrypto<Self::ReceiptVote>;
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
    type InternalVote = dre_ip::election::UnconfirmedVote<DreipGroup>;
    type ReceiptVote = dre_ip::election::ConfirmedVote<DreipGroup>;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalVote>,
    ) -> BallotCrypto<Self::ReceiptVote> {
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
    type InternalVote = dre_ip::election::UnconfirmedVote<DreipGroup>;
    type ReceiptVote = dre_ip::election::UnconfirmedVote<DreipGroup>;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalVote>,
    ) -> BallotCrypto<Self::ReceiptVote> {
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
    type InternalVote = dre_ip::election::ConfirmedVote<DreipGroup>;
    type ReceiptVote = dre_ip::election::ConfirmedVote<DreipGroup>;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalVote>,
    ) -> BallotCrypto<Self::ReceiptVote> {
        internal
    }
}
