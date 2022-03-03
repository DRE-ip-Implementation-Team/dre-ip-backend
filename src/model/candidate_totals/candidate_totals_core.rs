use dre_ip::CandidateTotals as DreipTotals;
use serde::{Deserialize, Serialize};

use crate::model::{election::DreipGroup, mongodb::Id};

/// Core candidate totals data, linked to a specific election->question->candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateTotalsCore {
    pub election_id: Id,
    pub question_id: Id,
    pub candidate_name: String,
    #[serde(flatten)]
    pub crypto: DreipTotals<DreipGroup>,
}

impl CandidateTotalsCore {
    /// Create new zero-totals.
    pub fn new(election_id: Id, question_id: Id, candidate_name: String) -> Self {
        Self {
            election_id,
            question_id,
            candidate_name,
            crypto: DreipTotals::default(),
        }
    }
}
