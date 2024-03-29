use std::fmt::Debug;

use dre_ip::{
    Ballot as DreipBallot, DreipGroup as DreipGroupTrait, DreipScalar, NoSecrets, SecretsPresent,
    VoteSecrets,
};
use mongodb::bson::{to_bson, Bson};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_unit_struct::{Deserialize_unit_struct, Serialize_unit_struct};

use crate::model::api::election::ReceiptError;
use crate::model::{
    api::{election::VerificationError, receipt::Receipt},
    common::election::{CandidateId, DreipGroup},
};

pub type BallotId = u32;
pub type BallotCrypto<S> = DreipBallot<CandidateId, DreipGroup, S>;

/// Trait for the ballot state, enforcing on the type level that secrets are present
/// if and only if the ballot is unconfirmed or audited.
pub trait BallotState: Copy + AsRef<[u8]> {
    /// Do we store the secrets internally?
    type InternalSecrets: Serialize + DeserializeOwned + Debug + Clone + VoteSecrets<DreipGroup>;

    /// Do we reveal the secrets in the receipt?
    type ExposedSecrets: Serialize + DeserializeOwned + Debug + Clone + VoteSecrets<DreipGroup>;

    /// Extra data to be included in a receipt of this type.
    type ReceiptData: Serialize + DeserializeOwned + Debug + Clone + PartialEq + Eq;

    /// Convert internal representation into receipt representation.
    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets>;

    /// Get the internal crypto with no secrets.
    fn remove_internal_secrets(
        internal: &BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<NoSecrets>;

    /// Get the external crypto with no secrets.
    fn remove_external_secrets(
        external: &BallotCrypto<Self::ExposedSecrets>,
    ) -> BallotCrypto<NoSecrets>;

    /// Retrieve the extra receipt data.
    fn receipt_data(internal: &BallotCrypto<Self::InternalSecrets>) -> Self::ReceiptData;

    /// Verify the extra receipt data.
    fn verify_receipt_data(receipt: &Receipt<Self>) -> Result<(), VerificationError>;
}

/// Extra candidate ID data for audited receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditExtraData {
    pub candidate: CandidateId,
}

impl<'a> From<&'a AuditExtraData> for Vec<u8> {
    fn from(data: &'a AuditExtraData) -> Self {
        data.candidate.clone().into_bytes()
    }
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
    type ReceiptData = NoSecrets;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal.confirm(None)
    }

    fn remove_internal_secrets(
        internal: &BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        internal.clone().confirm(None)
    }

    fn remove_external_secrets(
        external: &BallotCrypto<Self::ExposedSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        external.clone()
    }

    fn receipt_data(_: &BallotCrypto<Self::InternalSecrets>) -> Self::ReceiptData {
        NoSecrets(())
    }

    fn verify_receipt_data(_receipt: &Receipt<Self>) -> Result<(), VerificationError> {
        Ok(())
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
    type ReceiptData = AuditExtraData;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal
    }

    fn remove_internal_secrets(
        internal: &BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        internal.clone().confirm(None)
    }

    fn remove_external_secrets(
        external: &BallotCrypto<Self::ExposedSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        external.clone().confirm(None)
    }

    /// This assumes that the ballot is well-formed, i.e. there is a yes-candidate.
    /// If there is not, then the receipt is garbage and will not pass verification anyway,
    /// so we arbitrarily return the first candidate to avoid a panic.
    fn receipt_data(internal: &BallotCrypto<Self::InternalSecrets>) -> Self::ReceiptData {
        for (candidate, vote) in &internal.votes {
            if vote.secrets.v == <DreipGroup as DreipGroupTrait>::Scalar::one() {
                return AuditExtraData {
                    candidate: candidate.clone(),
                };
            }
        }

        // Technically, this could still panic if there are zero candidates,
        // but such ballots are impossible to construct unless you're *really* trying.
        AuditExtraData {
            candidate: internal.votes.keys().next().unwrap().clone(),
        }
    }

    fn verify_receipt_data(receipt: &Receipt<Self>) -> Result<(), VerificationError> {
        let correct_extra_data = Self::receipt_data(&receipt.crypto);
        if receipt.state_data == correct_extra_data {
            Ok(())
        } else {
            Err(VerificationError::Receipt(
                ReceiptError::RevealedCandidate {
                    ballot_id: receipt.ballot_id,
                    claimed_candidate: receipt.state_data.candidate.clone(),
                    true_candidate: correct_extra_data.candidate,
                },
            ))
        }
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
    type ReceiptData = NoSecrets;

    fn internal_to_receipt(
        internal: BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<Self::ExposedSecrets> {
        internal
    }

    fn remove_internal_secrets(
        internal: &BallotCrypto<Self::InternalSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        internal.clone()
    }

    fn remove_external_secrets(
        external: &BallotCrypto<Self::ExposedSecrets>,
    ) -> BallotCrypto<NoSecrets> {
        external.clone()
    }

    fn receipt_data(_: &BallotCrypto<Self::InternalSecrets>) -> Self::ReceiptData {
        NoSecrets(())
    }

    fn verify_receipt_data(_receipt: &Receipt<Self>) -> Result<(), VerificationError> {
        Ok(())
    }
}
