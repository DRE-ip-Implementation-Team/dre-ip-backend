mod base;
mod db;
mod finalizer;
mod metadata;

pub use base::Question;
pub use db::Election;
pub use finalizer::ElectionFinalizers;
pub use metadata::ElectionMetadata;

/// An [`crate::model::db::Election`] without an ID.
pub type NewElection = base::ElectionCore;
