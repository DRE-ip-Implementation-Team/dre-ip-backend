//! DB-compatible (e.g. de/serialisable) types.
//!
//! The types in this module are serialised in an DB-friendly way, e.g.:
//! - IDs and datetimes are serialised in `MongoDB`'s own format.

pub mod admin;
pub mod ballot;
pub mod candidate_totals;
pub mod election;
pub mod voter;
