use std::collections::HashMap;

use chrono::{DateTime, Utc};
use dre_ip::Election as DreipElection;
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::mongodb::{serde_string_map, Id};

use super::groups::Electorate;
use super::{CandidateID, DreipGroup, QuestionID};

/// Core election data, as stored in the database.
#[derive(Debug, Serialize, Deserialize)]
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
    /// A voter must be in at least one of these groups to vote on this question.
    pub groups: Vec<String>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<CandidateID>,
}
