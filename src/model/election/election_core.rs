use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::{Election as DreipElection, NoSecrets, PrivateKey};
use mongodb::bson::{self, serde_helpers::chrono_datetime_as_bson_datetime, Bson};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::mongodb::{serde_string_map, Id};

use super::electorate::Electorate;
use super::{CandidateID, DreipGroup, QuestionID};

/// Core election data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "S: Serialize", deserialize = "for<'a> S: Deserialize<'a>"))]
pub struct ElectionCore<S> {
    /// Top-level metadata.
    #[serde(flatten)]
    pub metadata: ElectionMetadata,
    /// Election electorates by name.
    pub electorates: HashMap<String, Electorate>,
    /// Election questions.
    #[serde(with = "serde_string_map")]
    pub questions: HashMap<QuestionID, Question>,
    /// Election cryptographic configuration.
    pub crypto: DreipElection<DreipGroup, S>,
}

impl ElectionCore<PrivateKey<DreipGroup>> {
    /// Create a new election.
    pub fn new(
        name: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        electorates: HashMap<String, Electorate>,
        questions: HashMap<QuestionID, Question>,
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

impl<S> ElectionCore<S> {
    /// Erase the secrets from this election.
    pub fn erase_secrets(self) -> ElectionCore<NoSecrets> {
        ElectionCore {
            metadata: self.metadata,
            electorates: self.electorates,
            questions: self.questions,
            crypto: self.crypto.erase_secrets(),
        }
    }
}

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionMetadata {
    /// Election name.
    pub name: String,
    /// Election state.
    pub state: ElectionState,
    /// Election start time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub start_time: DateTime<Utc>,
    /// Election end time.
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub end_time: DateTime<Utc>,
}

/// States in the Election lifecycle.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElectionState {
    /// Under construction, only visible to admins.
    Draft,
    /// Ready, in progress, or completed. Visible to all.
    Published,
    /// Completed, hidden by default, but retrievable by all.
    Archived,
}

// TODO: add similar impls for the ballot states and remove the string constants.
impl From<ElectionState> for Bson {
    fn from(state: ElectionState) -> Self {
        bson::to_bson(&state).unwrap() // Infallible.
    }
}

/// A single question.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    use chrono::Duration;

    use crate::model::election::ElectionSpec;

    use super::*;

    impl ElectionCore<PrivateKey<DreipGroup>> {
        pub fn draft_example() -> Self {
            ElectionSpec::future_example().into()
        }

        pub fn published_example() -> Self {
            let mut example: Self = ElectionSpec::current_example().into();
            example.metadata.state = ElectionState::Published;
            example
        }

        pub fn archived_example() -> Self {
            let mut example: Self = ElectionSpec::current_example().into();
            example.metadata.start_time = example.metadata.start_time - Duration::days(100);
            example.metadata.end_time = example.metadata.end_time - Duration::days(100);
            example.metadata.state = ElectionState::Archived;
            example
        }
    }
}
