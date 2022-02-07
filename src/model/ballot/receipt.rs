use serde::{Deserialize, Serialize};

use dre_ip::group::{DreipGroup, DreipPrivateKey, Serializable};

use super::ballot_core::{BallotCore, BallotState};

/// A receipt. Audited receipts will contain the secret values; any other type will not.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    /// The ballot itself.
    #[serde(flatten)]
    ballot: BallotCore,
    /// The signature.
    signature: Vec<u8>,
}

impl Receipt {
    /// Construct a receipt from the given ballot.
    pub fn from_ballot<G: DreipGroup>(mut ballot: BallotCore,
                                      signing_key: G::PrivateKey) -> Option<Self> {
        if let BallotState::Audited = ballot.state {
            // Audited receipts must contain all secret data.
            let secrets_present = ballot.votes
                .iter()
                .all(|vote| vote.secrets.is_some());
            if !secrets_present {
                return None;
            }
        } else {
            // Non-audited receipts must not contain secrets.
            for vote in ballot.votes.iter_mut() {
                vote.secrets = None;
            }
        }

        // Sign the receipt.
        let msg = ballot.to_bytes();
        let signature = signing_key.sign(&msg).to_bytes();

        // Construct the result.
        Some(Self {
            ballot,
            signature,
        })
    }
}
