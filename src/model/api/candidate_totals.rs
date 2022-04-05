use dre_ip::Serializable;
use serde::{Deserialize, Serialize};

use crate::model::{db::candidate_totals::CandidateTotals, mongodb::Id};

/// API-friendly representation of candidate totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateTotalsDesc {
    pub election_id: Id,
    pub question_id: Id,
    pub candidate_name: String,
    /// Vote tally.
    pub tally: String,
    /// Sum of randoms.
    pub r_sum: String,
}

impl From<CandidateTotals> for CandidateTotalsDesc {
    fn from(totals: CandidateTotals) -> Self {
        Self {
            election_id: totals.totals.election_id,
            question_id: totals.totals.question_id,
            candidate_name: totals.totals.candidate_name,
            tally: totals.totals.crypto.tally.to_bytestring(),
            r_sum: totals.totals.crypto.r_sum.to_bytestring(),
        }
    }
}
