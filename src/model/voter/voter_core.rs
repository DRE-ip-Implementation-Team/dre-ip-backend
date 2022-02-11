use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{
    mongodb::{serde_string_map, Id},
    sms::Sms,
};

/// Core voter user data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoterCore {
    /// Voter unique ID: their SMS number.
    pub sms: Sms,
    /// Maps election IDs to the IDs of questions the voter can answer for that election.
    /// This is populated according to their group constraints when they join groups,
    /// and depleted as they vote.
    #[serde(with = "serde_string_map")]
    pub allowed_questions: HashMap<Id, Vec<Id>>,
}

impl VoterCore {
    /// Create a new Voter.
    pub fn new(sms: Sms) -> Self {
        Self {
            sms,
            allowed_questions: HashMap::new(),
        }
    }
}

/// Example data for tests.
#[cfg(test)]
mod examples {
    use super::*;

    impl VoterCore {
        pub fn example() -> Self {
            Self {
                sms: Sms::example(),
                allowed_questions: HashMap::new(),
            }
        }
    }
}
