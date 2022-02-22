use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::Election as DreipElection;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::mongodb::{serde_string_map, Id};

use super::electorate::Electorate;
use super::{CandidateID, DreipGroup, QuestionID};

/// Core election data, as stored in the database.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct ElectionCore {
    /// Top-level metadata.
    #[serde(flatten)]
    pub metadata: ElectionMetadata,
    /// Election electorates by name.
    pub electorates: HashMap<String, Electorate>,
    /// Election questions.
    #[serde(with = "serde_string_map")]
    pub questions: HashMap<QuestionID, Question>,
    /// Election cryptographic configuration.
    pub crypto: DreipElection<DreipGroup>,
}

impl ElectionCore {
    /// Create a new election.
    pub fn new(
        metadata: ElectionMetadata,
        electorates: HashMap<String, Electorate>,
        questions: HashMap<QuestionID, Question>,
        rng: impl RngCore + CryptoRng,
    ) -> Self {
        let crypto = DreipElection::new(
            &[
                metadata.name.as_bytes(),
                &metadata.start_time.timestamp().to_le_bytes(),
                &metadata.end_time.timestamp().to_le_bytes(),
            ],
            rng,
        );

        Self {
            metadata,
            electorates,
            questions,
            crypto,
        }
    }
}

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionMetadata {
    /// Election name.
    pub name: String,
    /// Is the election finalised?
    pub finalised: bool,
    /// Election start time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub start_time: DateTime<Utc>,
    /// Election end time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub end_time: DateTime<Utc>,
}

/// A single question.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Question {
    /// Question unique ID.
    pub id: Id,
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these electorate groups to vote on this question.
    pub constraints: HashMap<String, HashSet<String>>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<CandidateID>,
}

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    use super::*;

    impl ElectionCore {
        pub fn example() -> Self {
            let electorates = [
                (Electorate::example1().name, Electorate::example1()),
                (Electorate::example2().name, Electorate::example2()),
            ]
            .into_iter()
            .collect();
            let questions = HashMap::default();
            Self::new(
                ElectionMetadata::example(),
                electorates,
                questions,
                thread_rng(),
            )
        }
    }

    impl ElectionMetadata {
        pub fn example() -> Self {
            Self {
                name: "".to_string(),
                finalised: false,
                start_time: Utc::now(),
                end_time: Utc::now(),
            }
        }
    }
}
