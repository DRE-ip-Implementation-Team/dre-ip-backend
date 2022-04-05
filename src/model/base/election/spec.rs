use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{
    base::{ElectionMetadata, ElectionState, Electorate},
    db::{NewElection, Question},
    mongodb::Id,
};

/// An election specification.
#[derive(Debug, Serialize, Deserialize)]
pub struct ElectionSpec {
    /// Election name.
    pub name: String,
    /// Election start time.
    pub start_time: DateTime<Utc>,
    /// Election end time.
    pub end_time: DateTime<Utc>,
    /// Election electorates.
    pub electorates: Vec<Electorate>,
    /// Election questions specifications.
    pub questions: Vec<QuestionSpec>,
}

impl From<ElectionSpec> for NewElection {
    fn from(spec: ElectionSpec) -> Self {
        let electorates = spec
            .electorates
            .into_iter()
            .map(|electorate| (electorate.name.clone(), electorate))
            .collect();
        Self::new(
            spec.name,
            spec.start_time,
            spec.end_time,
            electorates,
            spec.questions.into_iter().map(QuestionSpec::into).collect(),
            rand::thread_rng(),
        )
    }
}

impl From<ElectionSpec> for ElectionMetadata {
    fn from(spec: ElectionSpec) -> Self {
        Self {
            name: spec.name,
            state: ElectionState::Draft,
            start_time: spec.start_time,
            end_time: spec.end_time,
        }
    }
}

/// A question specification.
#[derive(Debug, Serialize, Deserialize)]
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
        pub fn current_example() -> Self {
            let start_time = Utc::today().and_hms(0, 0, 0);
            let end_time = start_time + Duration::days(30);
            Self {
                name: "Test Election 1".to_string(),
                start_time,
                end_time,
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![
                    QuestionSpec::example1(),
                    QuestionSpec::example2(),
                    QuestionSpec::example3(),
                ],
            }
        }

        pub fn future_example() -> Self {
            let start_time = Utc::today().and_hms(0, 0, 0) + Duration::days(30);
            let end_time = start_time + Duration::days(30);
            Self {
                name: "Test Election 2".to_string(),
                start_time,
                end_time,
                electorates: vec![Electorate::example1()],
                questions: vec![QuestionSpec::example1(), QuestionSpec::example2()],
            }
        }

        pub fn past_example() -> Self {
            let start_time = Utc::today().and_hms(0, 0, 0) - Duration::days(30);
            let end_time = start_time + Duration::days(7);
            Self {
                name: "Test Election 3".to_string(),
                start_time,
                end_time,
                electorates: vec![Electorate::example1()],
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

        pub fn example3() -> Self {
            Self {
                description: "Should CompSoc host a talk about Quantum Cryptography?".to_string(),
                constraints: HashMap::from_iter(vec![
                    (
                        "Societies".to_string(),
                        HashSet::from_iter(vec!["CompSoc".to_string()]),
                    ),
                    (
                        "Courses".to_string(),
                        HashSet::from_iter(vec!["CompSci".to_string()]),
                    ),
                ]),
                candidates: vec!["Yes".to_string(), "No".to_string()],
            }
        }
    }
}
