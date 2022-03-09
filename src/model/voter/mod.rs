pub use voter_core::{AllowedQuestions, HmacSha256};

mod voter_core;

/// A new voter ready for DB insertion is just a [`Voter`] without an ID, i.e. a `VoterCore`.
pub type NewVoter = voter_core::VoterCore;
