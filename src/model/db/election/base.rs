use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::{Election as DreipElection, NoSecrets, PrivateKey};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    base::{CandidateId, DreipGroup, ElectionMetadata, ElectionState, Electorate, QuestionId},
    mongodb::{serde_string_map, Id},
};

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
    pub questions: HashMap<QuestionId, Question>,
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
    pub candidates: Vec<CandidateId>,
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use crate::model::base::ElectionSpec;

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
