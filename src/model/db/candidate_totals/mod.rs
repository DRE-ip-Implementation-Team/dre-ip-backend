mod base;

/// A new [`CandidateTotals`] ready for DB insertion is just [`CandidateTotals`]
/// without an ID, i.e. `CandidateTotalsCore`.
pub type NewCandidateTotals = base::CandidateTotalsCore;

mod db;
pub use db::CandidateTotals;
