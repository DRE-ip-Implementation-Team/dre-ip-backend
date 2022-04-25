use dre_ip::DreipGroup as DreipGroupTrait;
use serde::{Deserialize, Serialize};

use crate::model::{common::election::DreipGroup, db::candidate_totals::CandidateTotals};

/// API-friendly representation of candidate totals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateTotalsDesc {
    pub election_id: u32,
    pub question_id: u32,
    pub candidate_name: String,
    /// Vote tally.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub tally: <DreipGroup as DreipGroupTrait>::Scalar,
    /// Sum of randoms.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub r_sum: <DreipGroup as DreipGroupTrait>::Scalar,
}

impl From<CandidateTotals> for CandidateTotalsDesc {
    fn from(totals: CandidateTotals) -> Self {
        Self {
            election_id: totals.totals.election_id,
            question_id: totals.totals.question_id,
            candidate_name: totals.totals.candidate_name,
            tally: totals.totals.crypto.tally,
            r_sum: totals.totals.crypto.r_sum,
        }
    }
}
