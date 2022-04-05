use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::{common::election::CandidateId, mongodb::Id};

/// A single question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// Question unique ID.
    pub id: Id,
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these electorate groups to vote on this question.
    pub constraints: HashMap<String, HashSet<String>>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<CandidateId>,
}
