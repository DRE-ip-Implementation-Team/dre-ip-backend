mod base;
pub use base::{AllowedQuestions, HmacSha256};

/// A [`crate::model::db::Voter`] without an ID.
pub type NewVoter = base::VoterCore;
