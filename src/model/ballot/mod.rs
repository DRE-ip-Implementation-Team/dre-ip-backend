pub use ballot_core::{BallotProof, BallotState, Vote, VoteProof, VoteSecrets};
pub use db::Ballot;

mod ballot_core;
mod db;

/// A new ballot ready for DB insertion is just a Ballot without an ID, i.e. a BallotCore.
pub type NewBallot = ballot_core::BallotCore;
