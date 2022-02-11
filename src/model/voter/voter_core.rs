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
    /// Maps election IDs to the IDs of the voter's groups for that election.
    // TODO combine with `election_voted` by precomputing allowed questions.
    #[serde(with = "serde_string_map")]
    pub election_groups: HashMap<Id, Vec<Id>>,
    /// Maps election IDs to the IDs of questions they have confirmed ballots on.
    #[serde(with = "serde_string_map")]
    pub election_voted: HashMap<Id, Vec<Id>>,
}

impl VoterCore {
    /// Create a new Voter.
    pub fn new(sms: Sms) -> Self {
        Self {
            sms,
            election_groups: HashMap::new(),
            election_voted: HashMap::new(),
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
                election_groups: HashMap::new(),
                election_voted: HashMap::new(),
            }
        }
    }
}
