use std::fmt::Debug;

use dre_ip::{Ballot as DreipBallot, NoSecrets, SecretsPresent, VoteSecrets};
use mongodb::bson::{to_bson, Bson};
use serde::{de::DeserializeOwned, Serialize};
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::common::election::{CandidateId, DreipGroup};

pub type BallotCrypto<S> = DreipBallot<CandidateId, DreipGroup, S>;

/// Trait for the ballot state, enforcing on the type level that secrets are present
/// if and only if the ballot is unconfirmed or audited.
pub trait BallotState: Copy + AsRef<[u8]> {
    /// Do we store the secrets internally?
    type InternalSecrets: Serialize + DeserializeOwned + Debug + Clone + VoteSecrets<DreipGroup>;
    /// Do we reveal the secrets in the receipt?
    type ExposedSecrets: Serialize + DeserializeOwned + Debug + Clone + VoteSecrets<DreipGroup>;
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
    type InternalSecrets = SecretsPresent<DreipGroup>;
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
    type InternalSecrets = SecretsPresent<DreipGroup>;
    type ExposedSecrets = SecretsPresent<DreipGroup>;

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
