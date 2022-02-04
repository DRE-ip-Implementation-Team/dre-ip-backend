use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{election::group::Group, mongodb::bson::Id, sms::Sms};

pub mod db;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voter {
    sms: Sms,
    election_groups: HashMap<Id, Vec<Group>>,
}

impl Voter {
    pub fn new(sms: Sms) -> Self {
        Self {
            sms,
            election_groups: HashMap::default(),
        }
    }

    pub fn sms(&self) -> &Sms {
        &self.sms
    }

    pub fn election_groups(&self) -> &HashMap<Id, Vec<Group>> {
        &self.election_groups
    }
}

#[cfg(test)]
mod examples {
    use super::*;

    use std::collections::HashMap;

    impl Voter {
        pub fn example() -> Self {
            Self {
                sms: Sms::example(),
                election_groups: HashMap::default(),
            }
        }
    }
}
