use chrono::{DateTime, Utc};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

use self::{electorate::Electorate, view::ElectionView};

use super::{ballot::Ballot, mongodb::bson::Id};

pub mod db;
pub mod electorate;
pub mod group;
pub mod view;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Election {
    #[serde(flatten)]
    short: ElectionView,
    electorates: Vec<Electorate>,
    questions: Vec<Question>,
    crypto: Crypto,
}

impl Election {
    pub fn new(
        name: String,
        finalised: bool,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        electorates: Vec<Electorate>,
        questions: Vec<Question>,
    ) -> Self {
        Self {
            short: ElectionView::new(name, finalised, start_time, end_time),
            electorates,
            questions,
            crypto: Crypto {
                private_key: (),
                public_key: (),
                g1: (),
                g2: (),
            },
        }
    }

    pub fn electorates(&self) -> &Vec<Electorate> {
        &self.electorates
    }

    pub fn questions(&self) -> &Vec<Question> {
        &self.questions
    }
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crypto {
    private_key: (),
    public_key: (),
    g1: (),
    g2: (),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Question {
    #[serde(flatten)]
    spec: QuestionSpec,
    ballots: Vec<Ballot>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct QuestionSpec {
    description: String,
    /// IDs refer to [`Group`]s, check if any user group is contained in this
    groups: Vec<Id>,
    candidates: Vec<String>,
}

impl From<QuestionSpec> for Question {
    fn from(spec: QuestionSpec) -> Self {
        Self {
            spec,
            ballots: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Candidate {
    name: String,
    #[serde(flatten)]
    sums: Sums,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Sums {
    tally: (),
    rsum: (),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionSpec {
    name: String,
    finalised: bool,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    start_time: DateTime<Utc>,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    end_time: DateTime<Utc>,
    electorates: Vec<Electorate>,
    questions: Vec<QuestionSpec>,
}

impl From<ElectionSpec> for Election {
    fn from(spec: ElectionSpec) -> Self {
        let ElectionSpec {
            name,
            finalised,
            start_time,
            end_time,
            electorates,
            questions: question_specs,
        } = spec;
        Self::new(
            name,
            finalised,
            start_time,
            end_time,
            electorates,
            question_specs.into_iter().map(QuestionSpec::into).collect(),
        )
    }
}

impl From<ElectionSpec> for ElectionView {
    fn from(spec: ElectionSpec) -> Self {
        let ElectionSpec {
            name,
            finalised,
            start_time,
            end_time,
            ..
        } = spec;
        Self::new(
            name,
            finalised,
            start_time,
            end_time,
        )
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    use chrono::{Duration, MIN_DATETIME};

    impl ElectionSpec {
        pub fn name(&self) -> &String {
            &self.name
        }

        pub fn example() -> Self {
            Self::finalised_example()
        }

        pub fn finalised_example() -> Self {
            Self {
                name: "Sports Clubs Elections".to_string(),
                finalised: true,
                start_time: MIN_DATETIME,
                end_time: MIN_DATETIME + Duration::days(30),
                electorates: vec![Electorate::example1(), Electorate::example2()],
                questions: vec![QuestionSpec::example()],
            }
        }

        pub fn unfinalised_example() -> Self {
            Self {
                name: "Sports Clubs Elections 2".to_string(),
                finalised: false,
                start_time: MIN_DATETIME,
                end_time: MIN_DATETIME + Duration::days(30),
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

    impl Candidate {
        pub fn example() -> Self {
            Self {
                name: "Chris Riches".to_string(),
                sums: Sums::default(),
            }
        }
    }
}
