use dre_ip::group::{DreipPrivateKey, Serializable};
use serde::{Deserialize, Serialize};

use crate::model::{
    election::Election,
    mongodb::Id,
};

use super::ballot_core::{BallotCrypto, BallotState};
use super::db::Ballot;

/// A receipt. Audited receipts will contain the secret values; any other type will not.
#[derive(Serialize, Deserialize)]
pub struct Receipt<S: BallotState> {
    /// The cryptographic data.
    #[serde(flatten)]
    pub crypto: BallotCrypto<S::ReceiptVote>,
    /// The ballot ID.
    pub id: Id,
    /// The current state of the ballot.
    pub state: S,
    /// The signature.
    pub signature: Vec<u8>,
}

impl<S: BallotState> Receipt<S> {
    /// Construct a receipt from the given ballot.
    pub fn from_ballot(ballot: Ballot<S>,
                       election: &Election) -> Self {
        // Convert the ballot from internal to receipt representation.
        let crypto = S::internal_to_receipt(ballot.ballot.crypto);

        // Sign the receipt.
        let mut msg = crypto.to_bytes();
        msg.extend(ballot.id.to_bytes());
        msg.extend(ballot.ballot.state.as_ref());
        let signature = election.crypto.private_key.sign(&msg).to_bytes();

        // Construct the result.
        Self {
            crypto,
            id: ballot.id,
            state: ballot.ballot.state,
            signature,
        }
    }
}
