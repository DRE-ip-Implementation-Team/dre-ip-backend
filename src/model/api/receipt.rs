use data_encoding::BASE32;
use dre_ip::{DreipGroup as DreipGroupTrait, DreipPrivateKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::{
    common::{
        ballot::{Audited, BallotCrypto, BallotState, Confirmed, Unconfirmed},
        election::DreipGroup,
    },
    db::{
        ballot::{AnyBallot, BallotCore},
        election::Election,
    },
};

pub type Signature = <DreipGroup as DreipGroupTrait>::Signature;

pub const CONFIRMATION_CODE_LENGTH: usize = 50;

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
    /// A hash of the IDs and the public crypto elements,
    /// encoded in base32 and truncated to 50 characters.
    pub confirmation_code: String,
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

        // Calculate the confirmation code.
        let confirmation_code = calc_confirmation_code(&ballot);

        // Convert the ballot from internal to receipt representation.
        let crypto = S::internal_to_receipt(ballot.crypto);

        // Sign the receipt.
        let mut msg = crypto.to_bytes();
        msg.extend(ballot.ballot_id.to_le_bytes());
        msg.extend(ballot.election_id.to_le_bytes());
        msg.extend(ballot.question_id.to_le_bytes());
        msg.extend(confirmation_code.as_bytes());
        msg.extend(ballot.state.as_ref());
        msg.extend(Into::<Vec<u8>>::into(&state_data));
        let signature = election.crypto.private_key.sign(&msg);

        // Construct the result.
        Self {
            crypto,
            ballot_id: ballot.ballot_id,
            election_id: ballot.election_id,
            question_id: ballot.question_id,
            confirmation_code,
            state: ballot.state,
            state_data,
            signature,
        }
    }
}

/// A stub receipt for an unconfirmed ballot.
/// We can't publicly reveal the full receipt since some of it is private information,
/// but we can still show the IDs and confirmation code on the bulletin board.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct UnconfirmedStub {
    /// Ballot ID.
    pub ballot_id: u32,
    /// Election ID.
    pub election_id: u32,
    /// Question ID.
    pub question_id: u32,
    /// A hash of the IDs and the public crypto elements,
    /// encoded in base32 and truncated to 50 characters.
    pub confirmation_code: String,
    /// The current state of the ballot.
    pub state: Unconfirmed,
    /// The signature.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub signature: Signature,
}

impl UnconfirmedStub {
    pub fn from_ballot(ballot: BallotCore<Unconfirmed>, election: &Election) -> Self {
        // Calculate the confirmation code.
        let confirmation_code = calc_confirmation_code(&ballot);

        // Sign the receipt.
        let mut msg = Vec::new();
        msg.extend(ballot.ballot_id.to_le_bytes());
        msg.extend(ballot.election_id.to_le_bytes());
        msg.extend(ballot.question_id.to_le_bytes());
        msg.extend(confirmation_code.as_bytes());
        msg.extend(ballot.state.as_ref());
        let signature = election.crypto.private_key.sign(&msg);

        // Construct the result.
        Self {
            ballot_id: ballot.ballot_id,
            election_id: ballot.election_id,
            question_id: ballot.question_id,
            confirmation_code,
            state: ballot.state,
            signature,
        }
    }
}

/// A receipt that is suitable for public display.
/// With the untagged representation, `Receipt<Audited>` and
/// `Receipt<Confirmed>` can both directly deserialize to this type.
#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PublicReceipt {
    Unconfirmed(UnconfirmedStub),
    Audited(Receipt<Audited>),
    Confirmed(Receipt<Confirmed>),
}

impl PublicReceipt {
    pub fn from_ballot(ballot: AnyBallot, election: &Election) -> Self {
        match ballot {
            AnyBallot::Unconfirmed(ballot) => {
                PublicReceipt::Unconfirmed(UnconfirmedStub::from_ballot(ballot.ballot, election))
            }
            AnyBallot::Audited(ballot) => {
                PublicReceipt::Audited(Receipt::from_ballot(ballot.ballot, election))
            }
            AnyBallot::Confirmed(ballot) => {
                PublicReceipt::Confirmed(Receipt::from_ballot(ballot.ballot, election))
            }
        }
    }
}

/// Calculate the confirmation code.
fn calc_confirmation_code<S: BallotState>(ballot: &BallotCore<S>) -> String {
    let mut hasher: Sha256 = Sha256::new();
    hasher.update(S::remove_internal_secrets(&ballot.crypto).to_bytes());
    hasher.update(ballot.ballot_id.to_le_bytes());
    hasher.update(ballot.election_id.to_le_bytes());
    hasher.update(ballot.question_id.to_le_bytes());
    let mut confirmation_code = BASE32.encode(&hasher.finalize());
    confirmation_code.truncate(CONFIRMATION_CODE_LENGTH);
    confirmation_code
}
