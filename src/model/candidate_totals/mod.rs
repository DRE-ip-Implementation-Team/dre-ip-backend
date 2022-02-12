pub use db::CandidateTotals;

mod candidate_totals_core;
mod db;

/// A new [`CandidateTotals`] ready for DB insertion is just [`CandidateTotals`]
/// without an ID, i.e. `CandidateTotalsCore`.
pub type NewCandidateTotals = candidate_totals_core::CandidateTotalsCore;
