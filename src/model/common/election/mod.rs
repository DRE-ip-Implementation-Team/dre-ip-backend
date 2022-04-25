mod electorate;
mod state;

pub use electorate::Electorate;
pub use state::ElectionState;

/// We implement our DRE-ip over the P-256 elliptic curve.
pub type DreipGroup = dre_ip::group::p256::NistP256;
/// Our election IDs are integers.
pub type ElectionId = u32;
/// Our question IDs are integers.
pub type QuestionId = u32;
/// Our candidate IDs (names) are strings.
pub type CandidateId = String;
