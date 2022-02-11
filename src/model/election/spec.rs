use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

use super::election_core::{ElectionCore, ElectionMetadata, Question};
use super::groups::Electorate;

/// An election specification.
#[derive(Debug, Serialize, Deserialize)]
pub struct ElectionSpec {
    /// Top-level metadata.
    #[serde(flatten)]
    pub metadata: ElectionMetadata,
    /// Election electorates.
    pub electorates: Vec<Electorate>,
    /// Election questions specifications.
    pub questions: Vec<QuestionSpec>,
}

impl From<ElectionSpec> for ElectionCore {
    fn from(spec: ElectionSpec) -> Self {
        let electorates = spec
            .electorates
            .into_iter()
            .map(|electorate| (electorate.name.clone(), electorate))
            .collect();
        Self::new(
            spec.metadata,
            electorates,
            spec.questions.into_iter().map(QuestionSpec::into).collect(),
            rand::thread_rng(),
        )
    }
}

impl From<ElectionSpec> for ElectionMetadata {
    fn from(spec: ElectionSpec) -> Self {
        Self {
            name: spec.metadata.name,
            finalised: spec.metadata.finalised,
            start_time: spec.metadata.start_time,
            end_time: spec.metadata.end_time,
        }
    }
}

/// A question specification.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct QuestionSpec {
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these groups to vote on this question.
    pub groups: Vec<Id>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<String>,
}

impl From<QuestionSpec> for (Id, Question) {
    fn from(spec: QuestionSpec) -> Self {
        let id = Id::new();
        let question = Question {
            id,
            description: spec.description,
            groups: spec.groups,
            candidates: spec.candidates,
        };

        (id, question)
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    use chrono::{Duration, MIN_DATETIME};

    impl ElectionSpec {
        pub fn finalised_example() -> Self {
            Self {
                metadata: ElectionMetadata {
                    name: "Sports Clubs Elections".to_string(),
                    finalised: true,
                    start_time: MIN_DATETIME,
                    end_time: MIN_DATETIME + Duration::days(30),
                },
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![QuestionSpec::example()],
            }
        }

        pub fn unfinalised_example() -> Self {
            Self {
                metadata: ElectionMetadata {
                    name: "Sports Clubs Elections 2".to_string(),
                    finalised: false,
                    start_time: MIN_DATETIME,
                    end_time: MIN_DATETIME + Duration::days(30),
                },
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![QuestionSpec::example()],
            }
        }
    }

    impl QuestionSpec {
        pub fn example() -> Self {
            Self {
                description: "Who should be captain of the Quidditch team?".to_string(),
                groups: vec![],
                candidates: vec!["Chris Riches".to_string()],
            }
        }
    }
}
