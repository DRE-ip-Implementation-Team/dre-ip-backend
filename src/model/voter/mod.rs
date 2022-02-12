pub use db::Voter;

mod db;
mod voter_core;

/// A new voter ready for DB insertion is just a [`Voter`] without an ID, i.e. a `VoterCore`.
pub type NewVoter = voter_core::VoterCore;
