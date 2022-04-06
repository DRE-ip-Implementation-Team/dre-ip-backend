use std::collections::HashMap;

use dre_ip::{
    BallotError, CandidateTotals, DreipPublicKey, VerificationError as InternalError, VoteError,
};
use serde::{Deserialize, Serialize};

use crate::model::{
    api::{
        candidate_totals::CandidateTotalsDesc, election::ElectionCrypto, id::ApiId,
        receipt::Receipt,
    },
    common::election::CandidateId,
    db::ballot::{Audited, Confirmed},
    mongodb::Id,
};

/// `Id` itself can't implement `AsRef<[u8]>`, so we convert to `Vec<u8>` first.
type BallotId = Vec<u8>;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum VerificationError {
    /// An individual ballot failed to verify.
    Ballot(BallotError<ApiId, String>),
    /// A receipt's signature was wrong.
    Receipt { ballot_id: ApiId },
    /// A candidate's tally or random sum failed to verify.
    Tally { candidate_id: String },
    /// The set of candidates does not match between the ballots
    /// and the proposed tallies.
    WrongCandidates,
}

impl From<InternalError<BallotId, CandidateId>> for VerificationError {
    fn from(err: InternalError<BallotId, CandidateId>) -> Self {
        match err {
            InternalError::Ballot(ballot_err) => {
                VerificationError::Ballot(match ballot_err {
                    BallotError::Vote(vote_err) => {
                        BallotError::Vote(VoteError {
                            // Convert bytes back into user-friendly ID.
                            // Unwrap safe since the bytes came from a valid ID originally.
                            ballot_id: Id::from_bytes(vote_err.ballot_id).unwrap().into(),
                            candidate_id: vote_err.candidate_id,
                        })
                    }
                    BallotError::BallotProof { ballot_id } => {
                        BallotError::BallotProof {
                            // Convert bytes back into user-friendly ID.
                            // Unwrap safe since the bytes came from a valid ID originally.
                            ballot_id: Id::from_bytes(ballot_id).unwrap().into(),
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
    pub election_crypto: ElectionCrypto,
    /// All audited receipts.
    pub audited_receipts: Vec<Receipt<Audited>>,
    /// All confirmed receipts.
    pub confirmed_receipts: Vec<Receipt<Confirmed>>,
    /// Claimed candidate totals.
    pub candidate_totals: HashMap<CandidateId, CandidateTotalsDesc>,
}

impl ElectionResults {
    /// Verify the election results.
    pub fn verify(&self) -> Result<(), VerificationError> {
        // Verify the confirmed ballots and candidate totals.
        let confirmed = self
            .confirmed_receipts
            .iter()
            .map(|r| (r.ballot_id.to_bytes(), r.crypto.clone()))
            .collect::<HashMap<_, _>>();
        let totals = self
            .candidate_totals
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
        dre_ip::verify_election(
            self.election_crypto.g1,
            self.election_crypto.g2,
            &confirmed,
            &totals,
        )?;

        // Verify the signatures of confirmed receipts.
        for receipt in self.confirmed_receipts.iter() {
            let mut msg = receipt.crypto.to_bytes();
            msg.extend(receipt.ballot_id.to_bytes());
            msg.extend(receipt.election_id.to_bytes());
            msg.extend(receipt.question_id.to_bytes());
            msg.extend(receipt.state.as_ref());
            if !self
                .election_crypto
                .public_key
                .verify(&msg, &receipt.signature)
            {
                return Err(VerificationError::Receipt {
                    ballot_id: receipt.ballot_id,
                });
            }
        }

        // Verify all the audited receipts.
        for receipt in self.audited_receipts.iter() {
            receipt
                .crypto
                .verify(
                    self.election_crypto.g1,
                    self.election_crypto.g2,
                    receipt.ballot_id.to_bytes(),
                )
                .map_err(InternalError::Ballot)?;

            let mut msg = receipt.crypto.to_bytes();
            msg.extend(receipt.ballot_id.to_bytes());
            msg.extend(receipt.election_id.to_bytes());
            msg.extend(receipt.question_id.to_bytes());
            msg.extend(receipt.state.as_ref());
            if !self
                .election_crypto
                .public_key
                .verify(&msg, &receipt.signature)
            {
                return Err(VerificationError::Receipt {
                    ballot_id: receipt.ballot_id,
                });
            }
        }

        Ok(())
    }
}
