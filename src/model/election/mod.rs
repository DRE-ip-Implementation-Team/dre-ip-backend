pub use db::Election;
pub use election_core::{Candidate, CandidateTotals, ElectionCrypto, ElectionMetadata, Question};
pub use groups::{Electorate, Group};
pub use spec::{ElectionSpec, QuestionSpec};

mod db;
mod election_core;
mod groups;
mod spec;

/// A new election ready for DB insertion is just an Election without an ID, i.e. an ElectionCore.
pub type NewElection = election_core::ElectionCore;
