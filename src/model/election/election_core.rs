use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dre_ip::{CandidateTotals, Election as DreipElection};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::mongodb::{Id, serde_string_map};

use super::{CandidateID, DreipGroup, QuestionID};
use super::groups::Electorate;

/// Core election data, as stored in the database.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct ElectionCore {
    /// Top-level metadata.
    #[serde(flatten)]
    pub metadata: ElectionMetadata,
    /// Election electorates.
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
            rng
        );

        Self {
            metadata,
            electorates,
            questions,
            crypto,
        }
    }

    /// Get a question by ID.
    pub fn question_totals(&mut self, question_id: Id) -> Option<HashMap<CandidateID, &mut CandidateTotals<DreipGroup>>> {
        self.questions
            .get_mut(&question_id)
            .map(|question| {
                let mut map = HashMap::with_capacity(question.candidates.len());

                for candidate in question.candidates.iter_mut() {
                    map.insert(candidate.name.clone(), &mut candidate.totals);
                }

                map
            })
    }
}

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Question {
    /// Question unique ID.
    pub id: Id,
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these groups to vote on this question.
    pub groups: Vec<Id>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<Candidate>,
}

impl Question {
    /// Get a candidate by name.
    pub fn candidate(&self, candidate_name: &str) -> Option<&Candidate> {
        self.candidates.iter().find(|c| c.name == candidate_name)
    }
}

/// A candidate: a possible answer to a specific question.
#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Candidate {
    /// User-facing name, also acts as a unique identifier.
    pub name: String,
    /// Cryptographic totals.
    #[serde(flatten)]
    pub totals: CandidateTotals<DreipGroup>,
}

impl Candidate {
    /// Create a new candidate from their name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            totals: CandidateTotals::default(),
        }
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl Candidate {
        pub fn example() -> Self {
            Self {
                name: "Chris Riches".to_string(),
                totals: CandidateTotals::default(),
            }
        }
    }
}
