use dre_ip::{DreipGroup as DreipGroupTrait, DreipPrivateKey};
use serde::{Deserialize, Serialize};

use crate::model::{
    common::{
        ballot::{Audited, BallotCrypto, BallotState, Confirmed},
        election::DreipGroup,
    },
    db::{
        ballot::{BallotCore, FinishedBallot},
        election::Election,
    },
};

pub type Signature = <DreipGroup as DreipGroupTrait>::Signature;

/// A receipt. Audited receipts will contain the secret values; any other type will not.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Receipt<S: BallotState> {
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: BallotCrypto<S::ExposedSecrets>,
    /// Ballot ID.
    pub ballot_id: u32,
    /// Election ID.
    pub election_id: u32,
    /// Question ID.
    pub question_id: u32,
    /// The current state of the ballot.
    pub state: S,
    /// Extra data specific to this ballot state.
    #[serde(flatten)]
    pub state_data: S::ReceiptData,
    /// The signature.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub signature: Signature,
}

impl<S: BallotState> Receipt<S>
where
    for<'a> &'a <S as BallotState>::ExposedSecrets: Into<Vec<u8>>,
    for<'a> &'a <S as BallotState>::ReceiptData: Into<Vec<u8>>,
{
    /// Construct a receipt from the given ballot.
    pub fn from_ballot(ballot: BallotCore<S>, election: &Election) -> Self {
        // Get any extra data.
        let state_data = S::receipt_data(&ballot.crypto);

        // Convert the ballot from internal to receipt representation.
        let crypto = S::internal_to_receipt(ballot.crypto);

        // Sign the receipt.
        let mut msg = crypto.to_bytes();
        msg.extend(ballot.ballot_id.to_le_bytes());
        msg.extend(ballot.election_id.to_le_bytes());
        msg.extend(ballot.question_id.to_le_bytes());
        msg.extend(ballot.state.as_ref());
        msg.extend(Into::<Vec<u8>>::into(&state_data));
        let signature = election.crypto.private_key.sign(&msg);

        // Construct the result.
        Self {
            crypto,
            ballot_id: ballot.ballot_id,
            election_id: ballot.election_id,
            question_id: ballot.question_id,
            state: ballot.state,
            state_data,
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
    pub fn from_finished_ballot(ballot: FinishedBallot, election: &Election) -> Self {
        match ballot {
            FinishedBallot::Audited(ballot) => {
                FinishedReceipt::Audited(Receipt::from_ballot(ballot.ballot, election))
            }
            FinishedBallot::Confirmed(ballot) => {
                FinishedReceipt::Confirmed(Receipt::from_ballot(ballot.ballot, election))
            }
        }
    }
}
