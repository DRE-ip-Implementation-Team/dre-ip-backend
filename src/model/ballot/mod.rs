pub use ballot_core::{
    Audited, BallotState, Confirmed, Unconfirmed, AUDITED, CONFIRMED, UNCONFIRMED,
};
pub use db::{Ballot, FinishedBallot};
pub use receipt::{Receipt, Signature};

mod ballot_core;
mod db;
mod receipt;
