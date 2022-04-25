use std::collections::HashMap;

use dre_ip::{CandidateTotals, DreipPublicKey, VerificationError as InternalError};
use serde::{Deserialize, Serialize};

use crate::model::{
    api::{candidate_totals::CandidateTotalsDesc, election::ElectionCrypto, receipt::Receipt},
    common::{
        ballot::{Audited, BallotId, BallotState, Confirmed},
        election::CandidateId,
    },
};

pub use dre_ip::{BallotError, VoteError};

/// `u32` itself can't implement `AsRef<[u8]>`, so we convert to `[u8; 4]` first.
pub type EffectiveBallotId = [u8; 4];

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum VerificationError {
    /// An individual ballot failed to verify.
    Ballot(BallotError<BallotId, String>),
    /// A receipt's signature was wrong.
    Receipt { ballot_id: BallotId },
    /// A candidate's tally or random sum failed to verify.
    Tally { candidate_id: String },
    /// The set of candidates does not match between the ballots
    /// and the proposed tallies.
    WrongCandidates,
}

impl From<InternalError<EffectiveBallotId, CandidateId>> for VerificationError {
    fn from(err: InternalError<EffectiveBallotId, CandidateId>) -> Self {
        match err {
            InternalError::Ballot(ballot_err) => {
                VerificationError::Ballot(match ballot_err {
                    BallotError::Vote(vote_err) => {
                        BallotError::Vote(VoteError {
                            // Convert bytes back into user-friendly ID.
                            ballot_id: u32::from_le_bytes(vote_err.ballot_id),
                            candidate_id: vote_err.candidate_id,
                        })
                    }
                    BallotError::BallotProof { ballot_id } => {
                        BallotError::BallotProof {
                            // Convert bytes back into user-friendly ID.
                            ballot_id: u32::from_le_bytes(ballot_id),
                        }
                    }
                })
            }
            InternalError::Tally { candidate_id } => VerificationError::Tally { candidate_id },
            InternalError::WrongCandidates => VerificationError::WrongCandidates,
        }
    }
}

/// All election results needed for verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionResults {
    /// Election cryptographic data needed for verification.
    pub election: ElectionCrypto,
    /// All audited receipts.
    pub audited: HashMap<BallotId, Receipt<Audited>>,
    /// All confirmed receipts.
    pub confirmed: HashMap<BallotId, Receipt<Confirmed>>,
    /// Claimed candidate totals.
    pub totals: HashMap<CandidateId, CandidateTotalsDesc>,
}

impl ElectionResults {
    /// Verify the election results.
    pub fn verify(&self) -> Result<(), VerificationError> {
        // Verify the confirmed ballots and candidate totals.
        let confirmed = self
            .confirmed
            .iter()
            .map(|(id, r)| (id.to_le_bytes(), r.crypto.clone()))
            .collect::<HashMap<_, _>>();
        let totals = self
            .totals
            .iter()
            .map(|(id, tot)| {
                (
                    id.clone(),
                    CandidateTotals {
                        tally: tot.tally,
                        r_sum: tot.r_sum,
                    },
                )
            })
            .collect::<HashMap<_, _>>();
        dre_ip::verify_election(self.election.g1, self.election.g2, &confirmed, &totals)?;

        // Verify the signatures of confirmed receipts.
        for receipt in self.confirmed.values() {
            verify_receipt_signature(receipt, &self.election)?;
        }

        // Verify all the audited receipts.
        for receipt in self.audited.values() {
            verify_receipt(receipt, &self.election)?;
        }

        Ok(())
    }
}

/// Verify an individual receipt.
pub fn verify_receipt<S>(
    receipt: &Receipt<S>,
    crypto: &ElectionCrypto,
) -> Result<(), VerificationError>
where
    S: BallotState,
    for<'a> &'a <S as BallotState>::ExposedSecrets: Into<Vec<u8>>,
    for<'a> &'a <S as BallotState>::ReceiptData: Into<Vec<u8>>,
{
    // Verify PWFs.
    receipt
        .crypto
        .verify(crypto.g1, crypto.g2, receipt.ballot_id.to_le_bytes())
        .map_err(InternalError::Ballot)?;

    // Verify signature.
    verify_receipt_signature(receipt, crypto)
}

/// Verify just the signature of an individual receipt.
pub fn verify_receipt_signature<S>(
    receipt: &Receipt<S>,
    crypto: &ElectionCrypto,
) -> Result<(), VerificationError>
where
    S: BallotState,
    for<'a> &'a <S as BallotState>::ExposedSecrets: Into<Vec<u8>>,
    for<'a> &'a <S as BallotState>::ReceiptData: Into<Vec<u8>>,
{
    let mut msg = receipt.crypto.to_bytes();
    msg.extend(receipt.ballot_id.to_le_bytes());
    msg.extend(receipt.election_id.to_le_bytes());
    msg.extend(receipt.question_id.to_le_bytes());
    msg.extend(receipt.state.as_ref());
    msg.extend(Into::<Vec<u8>>::into(&receipt.state_data));
    if crypto.public_key.verify(&msg, &receipt.signature) {
        Ok(())
    } else {
        Err(VerificationError::Receipt {
            ballot_id: receipt.ballot_id,
        })
    }
}
