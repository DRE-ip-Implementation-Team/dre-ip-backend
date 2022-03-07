use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use hmac::{digest::Output, Hmac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::{
    model::{
        mongodb::{serde_string_map, Id},
        sms::Sms,
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

/// The questions that a voter may answer for a particular election.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AllowedQuestions {
    /// For each allowed question, have they already confirmed a ballot?
    #[serde(with = "serde_string_map")]
    pub confirmed: HashMap<Id, bool>,
}

impl Deref for AllowedQuestions {
    type Target = HashMap<Id, bool>;

    fn deref(&self) -> &Self::Target {
        &self.confirmed
    }
}

impl DerefMut for AllowedQuestions {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.confirmed
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
