//! Types compatible with both API and DB.

mod admin;

/// A new admin ready for DB insertion is just an [`Admin`] without an ID, i.e. an `AdminCore`.
pub type NewAdmin = admin::AdminCore;

mod ballot;

pub use ballot::{Audited, BallotCore, BallotCrypto, BallotState, Confirmed, Unconfirmed};

mod voter;

pub use voter::{AllowedQuestions, HmacSha256};

/// A new voter ready for DB insertion is just a [`Voter`] without an ID, i.e. a `VoterCore`.
pub type NewVoter = voter::VoterCore;
