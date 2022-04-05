use dre_ip::{NoSecrets, PrivateKey};

use crate::model::base::DreipGroup;

mod db;
pub use db::Election;

/// Convenience wrapper. This should NEVER appear in an endpoint return type, or otherwise
/// be exposed to the outside world.
pub type ElectionWithSecrets = db::Election<PrivateKey<DreipGroup>>;

/// Convenience wrapper. Safe to expose to the outside world.
pub type ElectionNoSecrets = db::Election<NoSecrets>;

mod base;
pub use base::Question;

/// An [`crate::model::db::Election`] without an ID.
pub type NewElection = base::ElectionCore<PrivateKey<DreipGroup>>;

mod finalizer;
pub use finalizer::ElectionFinalizers;
