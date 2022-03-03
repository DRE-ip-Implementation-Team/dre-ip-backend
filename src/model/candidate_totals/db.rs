use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

use super::candidate_totals_core::CandidateTotalsCore;

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
