//! Types compatible with both API and DB.

mod admin;
pub use admin::NewAdmin;

mod voter;
pub use voter::{AllowedQuestions, HmacSha256, NewVoter};

mod election;
pub use election::{
    CandidateId, DreipGroup, ElectionMetadata, ElectionSpec, ElectionState, Electorate, QuestionId,
    QuestionSpec,
};
