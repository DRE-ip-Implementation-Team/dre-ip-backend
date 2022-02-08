pub use ballot_core::{Audited, AUDITED, BallotState, Confirmed, CONFIRMED, FinishedBallot, Receipt, Unconfirmed, UNCONFIRMED};
pub use db::{Ballot, DbBallot, NewBallot};

mod ballot_core;
mod db;
