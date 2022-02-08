use std::collections::HashMap;

use dre_ip::{Ballot as DreipBallot, CandidateTotals, Vote as DreipVote};
use dre_ip::group::{DreipGroup as DreipGroupTrait, DreipPrivateKey, Serializable};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::election::{CandidateID, DreipGroup};

/// Core ballot data, as stored in the database.
#[derive(Deserialize, Serialize)]
#[serde(rename = "camelCase")]
pub struct BallotCore<S: BallotState> {
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: DreipBallot<CandidateID, DreipGroup, S::InternalVote>,
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
    pub fn confirm(self, totals: Option<&mut HashMap<CandidateID, CandidateTotals<DreipGroup>>>) -> BallotCore<Confirmed> {
        BallotCore {
            crypto: self.crypto.confirm(totals),
            state: Confirmed,
        }
    }
}

/// A ballot that is either Confirmed or Audited.
/// With the untagged representation, `BallotCore<Audited>` and
/// `BallotCore<Confirmed>` can both directly deserialize to this type.
#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub enum FinishedBallot {
    Audited(BallotCore<Audited>),
    Confirmed(BallotCore<Confirmed>),
}

/// Trait for the ballot state, enforcing on the type level that secrets are present
/// if and only if the ballot is unconfirmed or audited.
pub trait BallotState {
    /// Is this state represented internally by a ConfirmedVote or an UnconfirmedVote?
    type InternalVote: DreipVote<DreipGroup> + Serialize + DeserializeOwned;
}

/// Marker type for unconfirmed ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Unconfirmed;

pub const UNCONFIRMED: &str = "Unconfirmed";

impl BallotState for Unconfirmed {
    type InternalVote = dre_ip::election::UnconfirmedVote<DreipGroup>;
}

/// Marker type for audited ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Audited;

pub const AUDITED: &str = "Audited";

impl BallotState for Audited {
    type InternalVote = dre_ip::election::UnconfirmedVote<DreipGroup>;
}

/// Marker type for confirmed ballots.
#[derive(Debug, Eq, PartialEq, Copy, Clone, Deserialize_unit_struct, Serialize_unit_struct)]
pub struct Confirmed;

pub const CONFIRMED: &str = "Confirmed";

impl BallotState for Confirmed {
    type InternalVote = dre_ip::election::ConfirmedVote<DreipGroup>;
}

/// A receipt. Audited receipts will contain the secret values; any other type will not.
#[derive(Serialize, Deserialize)]
pub struct Receipt<S: BallotState> {
    /// The ballot itself.
    #[serde(flatten)]
    ballot: BallotCore<S>,
    /// The signature.
    signature: Vec<u8>,
}

impl<S: BallotState> Receipt<S> {
    /// Construct a receipt from the given ballot.
    pub fn from_ballot(ballot: BallotCore<S>,
                       signing_key: <DreipGroup as DreipGroupTrait>::PrivateKey) -> Option<Self> {
        // Sign the receipt.
        let msg = ballot.crypto.to_bytes();
        let signature = signing_key.sign(&msg).to_bytes();

        // Construct the result.
        Some(Self {
            ballot,
            signature,
        })
    }
}
