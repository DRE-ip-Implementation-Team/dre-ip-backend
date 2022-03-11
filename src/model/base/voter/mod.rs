mod base;
pub use base::{AllowedQuestions, HmacSha256};

/// A new voter ready for DB insertion is just a [`Voter`] without an ID, i.e. a `VoterCore`.
pub type NewVoter = base::VoterCore;
