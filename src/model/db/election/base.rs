use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::Election as DreipElection;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    common::election::{
        CandidateId, DreipGroup, ElectionId, ElectionState, Electorate, QuestionId,
    },
    mongodb::serde_string_map,
};

use super::metadata::ElectionMetadata;

/// Core election data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Election {
    /// Unique ID.
    #[serde(rename = "_id")]
    pub id: ElectionId,
    /// Top-level metadata.
    #[serde(flatten)]
    pub metadata: ElectionMetadata,
    /// Election electorates by name.
    pub electorates: HashMap<String, Electorate>,
    /// Election questions.
    #[serde(with = "serde_string_map")]
    pub questions: HashMap<QuestionId, Question>,
    /// Election cryptographic configuration.
    pub crypto: DreipElection<DreipGroup>,
}

impl Election {
    /// Create a new election.
    pub fn new(
        id: ElectionId,
        name: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        electorates: HashMap<String, Electorate>,
        questions: HashMap<QuestionId, Question>,
        rng: impl RngCore + CryptoRng,
    ) -> Self {
        let crypto = DreipElection::new(
            &[
                name.as_bytes(),
                &start_time.timestamp().to_le_bytes(),
                &end_time.timestamp().to_le_bytes(),
            ],
            rng,
        );

        Self {
            id,
            metadata: ElectionMetadata {
                name,
                state: ElectionState::Draft,
                start_time,
                end_time,
            },
            electorates,
            questions,
            crypto,
        }
    }
}

/// A single question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    /// Unique ID.
    pub id: QuestionId,
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these electorate groups to vote on this question.
    pub constraints: HashMap<String, HashSet<String>>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<CandidateId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::api::election::ElectionSpec;

    impl Election {
        pub fn draft_example() -> Self {
            let mut rng = rand::thread_rng();
            ElectionSpec::future_example().into_election(rng.next_u32(), rng)
        }

        pub fn published_example() -> Self {
            let mut rng = rand::thread_rng();
            let mut example: Self =
                ElectionSpec::current_example().into_election(rng.next_u32(), rng);
            example.metadata.state = ElectionState::Published;
            example
        }

        pub fn archived_example() -> Self {
            let mut rng = rand::thread_rng();
            let mut example: Self = ElectionSpec::past_example().into_election(rng.next_u32(), rng);
            example.metadata.state = ElectionState::Archived;
            example
        }
    }
}
