pub use db::Election;
pub use election_core::{ElectionMetadata, Question};
pub use groups::Electorate;
pub use spec::{ElectionSpec, QuestionSpec};

mod db;
mod election_core;
mod groups;
mod spec;

/// A new election ready for DB insertion is just an [`Election`] without an ID, i.e. an `ElectionCore`.
pub type NewElection = election_core::ElectionCore;

/// We implement our DRE-ip over the P-256 elliptic curve.
pub type DreipGroup = dre_ip::group::p256::NistP256;
/// Our question IDs are [`mongodb::bson::oid::ObjectId`]s.
pub type QuestionID = crate::model::mongodb::Id;
/// Our candidate IDs (names) are strings.
pub type CandidateID = String;
