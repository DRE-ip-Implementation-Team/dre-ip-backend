use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use dre_ip::DreipGroup as DreipGroupTrait;
use mongodb::bson::{doc, Document};
use rocket::{FromFormField, UriDisplayQuery};
use serde::{Deserialize, Serialize};

use crate::model::{
    common::election::{DreipGroup, ElectionState, Electorate},
    db::election::{Election, ElectionMetadata, Question},
};

/// An API-friendly representation of the relationship between the current time
/// and an election's start/end times.
/// This obviously goes out of date if stored, so only use it transiently.
#[derive(
    Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize, FromFormField, UriDisplayQuery,
)]
pub enum ElectionTiming {
    /// The start time is in the future.
    Future,
    /// The start time is in the past and the end time in the future.
    Current,
    /// The end time is in the past.
    Past,
}

impl ElectionTiming {
    /// Get the status of the given election.
    pub fn for_metadata(election: &ElectionMetadata) -> Self {
        let now = Utc::now();
        if now < election.start_time {
            ElectionTiming::Future
        } else if now < election.end_time {
            ElectionTiming::Current
        } else {
            ElectionTiming::Past
        }
    }

    /// Get the mongodb filter for elections of this status.
    pub fn filter(&self) -> Document {
        let now = Utc::now();
        match self {
            ElectionTiming::Future => doc! { "start_time": { "$gt": now } },
            ElectionTiming::Current => doc! {
                "start_time": { "$lte": now },
                "end_time": { "$gt": now },
            },
            ElectionTiming::Past => doc! { "end_time": { "$lte": now } },
        }
    }
}

/// An API-friendly election description, containing no sensitive data or weird formats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionDescription {
    /// Election unique ID.
    pub id: u32,
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
    pub questions: HashMap<u32, QuestionDescription>,
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
            .questions
            .into_iter()
            .map(|(id, q)| (id, q.into()))
            .collect();

        Self {
            id: election.id,
            name: election.metadata.name,
            state: election.metadata.state,
            start_time: election.metadata.start_time,
            end_time: election.metadata.end_time,
            electorates: election.electorates,
            questions,
            crypto: ElectionCrypto {
                g1: election.crypto.g1,
                g2: election.crypto.g2,
                public_key: election.crypto.public_key,
            },
        }
    }
}

/// A summary of an election, shorter than the full `ElectionDescription`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ElectionSummary {
    /// Election unique ID.
    pub id: u32,
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
            id: election.id,
            name: election.metadata.name,
            state: election.metadata.state,
            start_time: election.metadata.start_time,
            end_time: election.metadata.end_time,
        }
    }
}

/// An API-friendly description of a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionDescription {
    /// Question unique ID, in API-friendly form.
    pub id: u32,
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
            id: question.id,
            description: question.description,
            constraints: question.constraints,
            candidates: question.candidates,
        }
    }
}
