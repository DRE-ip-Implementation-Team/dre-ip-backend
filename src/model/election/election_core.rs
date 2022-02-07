use chrono::{DateTime, Utc};
use mongodb::bson::serde_helpers::chrono_datetime_as_bson_datetime;
use serde::{Deserialize, Serialize};

use crate::model::{ballot::Ballot, mongodb::Id};

use super::groups::Electorate;

/// Core election data, as stored in the database.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct ElectionCore {
    /// Top-level metadata.
    #[serde(flatten)]
    metadata: ElectionMetadata,
    /// Election electorates.
    electorates: Vec<Electorate>,
    /// Election questions.
    questions: Vec<Question>,
    /// Election cryptographic configuration.
    crypto: ElectionCrypto,
}

impl ElectionCore {
    /// Create a new election.
    pub fn new(
        metadata: ElectionMetadata,
        electorates: Vec<Electorate>,
        questions: Vec<Question>,
    ) -> Self {
        Self {
            metadata,
            electorates,
            questions,
            crypto: ElectionCrypto::default(),
        }
    }

    /// Get the metadata.
    pub fn metadata(&self) -> &ElectionMetadata {
        &self.metadata
    }

    /// Get the electorates.
    pub fn electorates(&self) -> &Vec<Electorate> {
        &self.electorates
    }

    /// Get the questions.
    pub fn questions(&self) -> &Vec<Question> {
        &self.questions
    }
}

/// A view on just the election's top-level metadata.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Election cryptographic configuration as per the DRE-ip protocol.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ElectionCrypto {
    private_key: Vec<u8>,
    public_key: Vec<u8>,
    g1: Vec<u8>,
    g2: Vec<u8>,
}

/// A single question.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Question {
    /// Question text.
    pub description: String,
    /// A voter must be in at least one of these groups to vote on this question.
    pub groups: Vec<Id>,
    /// Candidates / possible answers for this question.
    pub candidates: Vec<Candidate>,
    /// Ballots cast on this question.
    pub ballots: Vec<Ballot>,
}

/// A candidate: a possible answer to a specific question.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Candidate {
    /// User-facing name.
    pub name: String,
    /// Cryptographic totals.
    #[serde(flatten)]
    pub totals: CandidateTotals,
}

impl Candidate {
    /// Create a new candidate from their name.
    pub fn new(name: String) -> Self {
        Self {
            name,
            totals: CandidateTotals::default(),
        }
    }
}

/// Cryptographic totals for a candidate as per the DRE-ip protocol.
#[derive(Debug, Serialize, Deserialize)]
pub struct CandidateTotals {
    /// Total tally of yes votes.
    pub tally: Vec<u8>,
    /// Sum of secret random values.
    pub r_sum: Vec<u8>,
}

impl Default for CandidateTotals {
    fn default() -> Self {
        Self {
            tally: vec![0],
            r_sum: vec![0],
        }
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl Candidate {
        pub fn example() -> Self {
            Self {
                name: "Chris Riches".to_string(),
                totals: CandidateTotals::default(),
            }
        }
    }
}
