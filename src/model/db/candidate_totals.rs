use std::ops::{Deref, DerefMut};

use dre_ip::CandidateTotals as DreipTotals;
use serde::{Deserialize, Serialize};

use crate::model::{common::election::DreipGroup, mongodb::Id};

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

/// A new [`CandidateTotals`] ready for DB insertion is just [`CandidateTotals`]
/// without an ID, i.e. `CandidateTotalsCore`.
pub type NewCandidateTotals = CandidateTotalsCore;

/// Candidate totals from the database, with their unique ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateTotals {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub totals: CandidateTotalsCore,
}

impl Deref for CandidateTotals {
    type Target = CandidateTotalsCore;

    fn deref(&self) -> &Self::Target {
        &self.totals
    }
}

impl DerefMut for CandidateTotals {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.totals
    }
}
