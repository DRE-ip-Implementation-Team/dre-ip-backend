use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::DreipGroup as DreipGroupTrait;
use serde::{Deserialize, Serialize};

use crate::model::{
    api::id::ApiId,
    common::election::{DreipGroup, ElectionState, Electorate},
    db::election::{Election, Question},
};

/// An API-friendly election description, containing no sensitive data or weird formats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionDescription {
    /// Election unique ID.
    pub id: ApiId,
    /// Election name.
    pub name: String,
    /// Election state.
    pub state: ElectionState,
    /// Election start time.
    pub start_time: DateTime<Utc>,
    /// Election end time.
    pub end_time: DateTime<Utc>,
    /// Election electorates by name.
    pub electorates: HashMap<String, Electorate>,
    /// Election questions.
    pub questions: HashMap<ApiId, QuestionDescription>,
    /// Election cryptographic configuration.
    pub crypto: ElectionCrypto,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionCrypto {
    /// First generator.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub g1: <DreipGroup as DreipGroupTrait>::Point,
    /// Second generator.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub g2: <DreipGroup as DreipGroupTrait>::Point,
    /// Verification key.
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub public_key: <DreipGroup as DreipGroupTrait>::PublicKey,
}

impl From<Election> for ElectionDescription {
    fn from(election: Election) -> Self {
        let questions = election
            .election
            .questions
            .into_iter()
            .map(|(id, q)| (id.into(), q.into()))
            .collect();

        Self {
            id: election.id.into(),
            name: election.election.metadata.name,
            state: election.election.metadata.state,
            start_time: election.election.metadata.start_time,
            end_time: election.election.metadata.end_time,
            electorates: election.election.electorates,
            questions,
            crypto: ElectionCrypto {
                g1: election.election.crypto.g1,
                g2: election.election.crypto.g2,
                public_key: election.election.crypto.public_key,
            },
        }
    }
}

/// A summary of an election, shorter than the full `ElectionDescription`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionSummary {
    /// Election unique ID.
    pub id: ApiId,
    /// Election name.
    pub name: String,
    /// Election state.
    pub state: ElectionState,
    /// Election start time.
    pub start_time: DateTime<Utc>,
    /// Election end time.
    pub end_time: DateTime<Utc>,
}

impl From<Election> for ElectionSummary {
    fn from(election: Election) -> Self {
        Self {
            id: election.id.into(),
            name: election.election.metadata.name,
            state: election.election.metadata.state,
            start_time: election.election.metadata.start_time,
            end_time: election.election.metadata.end_time,
        }
    }
}

/// An API-friendly description of a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionDescription {
    /// Question unique ID, in API-friendly form.
    pub id: ApiId,
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these electorate groups to vote on this question.
    pub constraints: HashMap<String, HashSet<String>>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<String>,
}

impl From<Question> for QuestionDescription {
    fn from(question: Question) -> Self {
        Self {
            id: question.id.into(),
            description: question.description,
            constraints: question.constraints,
            candidates: question.candidates,
        }
    }
}
