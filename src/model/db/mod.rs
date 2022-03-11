//! DB-compatible (e.g. de/serialisable) types.
//!
//! The types in this module are serialised in an DB-friendly way, e.g.:
//!
//! - IDs and datetimes are serialised in MongoDB's own format.

mod admin;
pub use admin::Admin;

mod ballot;
pub use ballot::{
    Audited, Ballot, BallotCore, BallotCrypto, BallotState, Confirmed, FinishedBallot, Unconfirmed,
};

mod candidate_totals;
pub use candidate_totals::{CandidateTotals, NewCandidateTotals};

mod election;
pub use election::{Election, ElectionNoSecrets, ElectionWithSecrets, NewElection, Question};

mod voter;
pub use voter::Voter;

mod receipt;
pub use receipt::{FinishedReceipt, Receipt, Signature};
