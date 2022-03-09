mod admin;
pub use admin::Admin;

mod ballot;
pub use ballot::{Ballot, FinishedBallot};

mod candidate_totals;
pub use candidate_totals::CandidateTotals;

mod election;
pub use election::Election;

use super::election::DreipGroup;
use dre_ip::{NoSecrets, PrivateKey};

/// Convenience wrapper. This should NEVER appear in an endpoint return type, or otherwise
/// be exposed to the outside world.
pub type ElectionWithSecrets = election::Election<PrivateKey<DreipGroup>>;

/// Convenience wrapper. Safe to expose to the outside world.
pub type ElectionNoSecrets = election::Election<NoSecrets>;

mod voter;

pub use voter::Voter;
