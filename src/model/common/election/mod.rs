mod electorate;
mod question;
mod state;

pub use electorate::Electorate;
pub use question::Question;
pub use state::ElectionState;

/// We implement our DRE-ip over the P-256 elliptic curve.
pub type DreipGroup = dre_ip::group::p256::NistP256;
/// Our question IDs are [`mongodb::bson::oid::ObjectId`]s.
pub type QuestionId = crate::model::mongodb::Id;
/// Our candidate IDs (names) are strings.
pub type CandidateId = String;
