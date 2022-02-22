use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

use super::election_core::{ElectionCore, ElectionMetadata, Question};
use super::electorate::Electorate;

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
    /// A voter must be in at least one of these electorate groups to vote on this question.
    pub constraints: HashMap<String, HashSet<String>>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<String>,
}

impl From<QuestionSpec> for (Id, Question) {
    fn from(spec: QuestionSpec) -> Self {
        let id = Id::new();
        let question = Question {
            id,
            description: spec.description,
            constraints: spec.constraints,
            candidates: spec.candidates,
        };

        (id, question)
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    use chrono::{Duration, Utc};

    impl ElectionSpec {
        pub fn finalised_example() -> Self {
            let start_time = Utc::today().and_hms(0, 0, 0);
            let end_time = start_time + Duration::days(30);
            Self {
                metadata: ElectionMetadata {
                    name: "Sports Clubs Elections".to_string(),
                    finalised: true,
                    start_time,
                    end_time,
                },
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![QuestionSpec::example1(), QuestionSpec::example2()],
            }
        }

        pub fn unfinalised_example() -> Self {
            let start_time = Utc::today().and_hms(0, 0, 0) + Duration::days(30);
            let end_time = start_time + Duration::days(30);
            Self {
                metadata: ElectionMetadata {
                    name: "Sports Clubs Elections 2".to_string(),
                    finalised: false,
                    start_time,
                    end_time,
                },
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![QuestionSpec::example1(), QuestionSpec::example2()],
            }
        }
    }

    impl QuestionSpec {
        pub fn example1() -> Self {
            Self {
                description: "Who should be captain of the Quidditch team?".to_string(),
                constraints: HashMap::from_iter(vec![(
                    "Societies".to_string(),
                    HashSet::from_iter(vec!["Quidditch".to_string()]),
                )]),
                candidates: vec!["Chris Riches".to_string(), "Parry Hotter".to_string()],
            }
        }

        pub fn example2() -> Self {
            Self {
                description: "Who should be president of Warwick Extreme Moongolf?".to_string(),
                constraints: HashMap::from_iter(vec![(
                    "Societies".to_string(),
                    HashSet::from_iter(vec!["Moongolf".to_string()]),
                )]),
                candidates: vec!["John Smith".to_string(), "Jane Doe".to_string()],
            }
        }
    }
}
