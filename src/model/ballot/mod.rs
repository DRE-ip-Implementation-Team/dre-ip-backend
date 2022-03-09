mod ballot_core;
mod receipt;

pub use ballot_core::{Audited, BallotCore, BallotState, Confirmed, Unconfirmed};
pub use receipt::{FinishedReceipt, Receipt, Signature};
