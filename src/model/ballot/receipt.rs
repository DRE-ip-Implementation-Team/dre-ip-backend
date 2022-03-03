use dre_ip::{DreipGroup as DreipGroupTrait, DreipPrivateKey};
use serde::{Deserialize, Serialize};

use crate::model::{
    election::{DreipGroup, ElectionWithSecrets},
    mongodb::Id,
};

use super::ballot_core::{Audited, BallotCrypto, BallotState, Confirmed};
use super::db::{Ballot, FinishedBallot};

pub type Signature = <DreipGroup as DreipGroupTrait>::Signature;

/// A receipt. Audited receipts will contain the secret values; any other type will not.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Receipt<S: BallotState> {
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: BallotCrypto<S::ExposedSecrets>,
    /// Ballot ID.
    pub ballot_id: Id,
    /// Election ID.
    pub election_id: Id,
    /// Question ID.
    pub question_id: Id,
    /// The current state of the ballot.
    pub state: S,
    /// The signature.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub signature: Signature,
}

impl<S: BallotState> Receipt<S>
where
    for<'a> &'a <S as BallotState>::ExposedSecrets: Into<Vec<u8>>,
{
    /// Construct a receipt from the given ballot.
    pub fn from_ballot(ballot: Ballot<S>, election: &ElectionWithSecrets) -> Self {
        // Convert the ballot from internal to receipt representation.
        let crypto = S::internal_to_receipt(ballot.ballot.crypto);

        // Sign the receipt.
        let mut msg = crypto.to_bytes();
        msg.extend(ballot.id.to_bytes());
        msg.extend(ballot.election_id.to_bytes());
        msg.extend(ballot.question_id.to_bytes());
        msg.extend(ballot.ballot.state.as_ref());
        let signature = election.crypto.private_key.sign(&msg);

        // Construct the result.
        Self {
            crypto,
            ballot_id: ballot.id,
            election_id: ballot.election_id,
            question_id: ballot.question_id,
            state: ballot.ballot.state,
            signature,
        }
    }
}

/// A receipt that is either Confirmed or Audited.
/// With the untagged representation, `Receipt<Audited>` and
/// `Receipt<Confirmed>` can both directly deserialize to this type.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum FinishedReceipt {
    Audited(Receipt<Audited>),
    Confirmed(Receipt<Confirmed>),
}

impl FinishedReceipt {
    pub fn from_finished_ballot(ballot: FinishedBallot, election: &ElectionWithSecrets) -> Self {
        match ballot {
            FinishedBallot::Audited(ballot) => {
                FinishedReceipt::Audited(Receipt::from_ballot(ballot, election))
            }
            FinishedBallot::Confirmed(ballot) => {
                FinishedReceipt::Confirmed(Receipt::from_ballot(ballot, election))
            }
        }
    }
}
