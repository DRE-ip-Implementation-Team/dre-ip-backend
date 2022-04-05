use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::{common::candidate_totals::CandidateTotalsCore, mongodb::Id};

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
