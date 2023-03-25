use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};

use crate::model::{
    common::election::{ElectionId, ElectionState, Electorate, QuestionId},
    db::election::{Election, ElectionMetadata, Question},
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

impl ElectionSpec {
    /// Convert this spec into a proper Election with unique IDs.
    pub fn into_election(self, election_id: ElectionId, rng: impl RngCore + CryptoRng) -> Election {
        let electorates = self
            .electorates
            .into_iter()
            .map(|electorate| (electorate.name.clone(), electorate))
            .collect();
        Election::new(
            election_id,
            self.name,
            self.start_time,
            self.end_time,
            electorates,
            self.questions
                .into_iter()
                .enumerate()
                .map(|(i, q)| {
                    let question_id = 1 + QuestionId::try_from(i).expect("usize to u32");
                    (question_id, q.into_question(question_id))
                })
                .collect(),
            rng,
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

impl QuestionSpec {
    /// Convert this spec into a question with the given unique ID.
    pub fn into_question(self, id: QuestionId) -> Question {
        Question {
            id,
            description: self.description,
            constraints: self.constraints,
            candidates: self.candidates,
        }
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    use chrono::{Duration, Timelike, Utc};

    macro_rules! midnight_today {
        () => {{
            Utc::now()
                .with_hour(0)
                .and_then(|t| t.with_minute(0))
                .and_then(|t| t.with_second(0))
                .and_then(|t| t.with_nanosecond(0))
                .unwrap()
        }};
    }

    impl ElectionSpec {
        pub fn current_example() -> Self {
            let start_time = midnight_today!();
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
                    QuestionSpec::example4(),
                ],
            }
        }

        pub fn future_example() -> Self {
            let start_time = midnight_today!() + Duration::days(30);
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
            let start_time = midnight_today!() - Duration::days(30);
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

        pub fn example4() -> Self {
            Self {
                description: "Should this question really be open to everyone?".to_string(),
                constraints: HashMap::new(),
                candidates: vec!["Definitely".to_string(), "Absolutely".to_string()],
            }
        }
    }
}
