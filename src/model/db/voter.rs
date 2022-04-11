use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use hmac::{digest::Output, Hmac};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    model::{
        api::sms::Sms,
        common::allowed_questions::AllowedQuestions,
        mongodb::{serde_string_map, Id},
    },
    Config,
};

pub type HmacSha256 = Hmac<Sha256>;

/// Core voter user data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoterCore {
    /// Voter unique ID: the HMAC of their SMS number.
    pub sms_hmac: Output<HmacSha256>,
    /// Maps election IDs to the IDs of questions the voter can answer for that election.
    /// This is populated according to their group constraints when they join an election.
    #[serde(with = "serde_string_map")]
    pub allowed_questions: HashMap<Id, AllowedQuestions>,
}

impl VoterCore {
    /// Create a new Voter.
    pub fn new(sms: Sms, config: &Config) -> Self {
        Self {
            // Do not directly store potentially sensitive phone number data
            sms_hmac: sms.into_hmac(config),
            allowed_questions: HashMap::new(),
        }
    }
}

/// A voter without an ID.
pub type NewVoter = VoterCore;

/// A voter user from the database, with its unique ID.
#[derive(Debug, Serialize, Deserialize)]
pub struct Voter {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub voter: VoterCore,
}

impl Deref for Voter {
    type Target = VoterCore;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

impl DerefMut for Voter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.voter
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl VoterCore {
        pub fn example(config: &Config) -> Self {
            Self {
                sms_hmac: Sms::example().into_hmac(config),
                allowed_questions: HashMap::new(),
            }
        }
    }
}
