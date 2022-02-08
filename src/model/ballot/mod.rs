pub use ballot_core::{Audited, AUDITED, BallotState, Confirmed, CONFIRMED, FinishedBallot, Receipt, Unconfirmed, UNCONFIRMED};
pub use db::{Ballot, DbBallot};

mod ballot_core;
mod db;

/// We generate ballot IDs ourself rather than letting the database do it, but
/// we still want to strip IDs before sending ballots to the user, since the IDs
/// are sensitive information that allow confirming/auditing any ballot you know
/// the ID of.
/// This type alias acts as an ID-less ballot.
pub type NewBallot = ballot_core::BallotCore<Unconfirmed>;
