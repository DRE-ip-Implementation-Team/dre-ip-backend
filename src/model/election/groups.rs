use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

/// An electorate is a potentially mutually-exclusive set of logically related groups.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Electorate {
    /// Electorate name.
    name: String,
    /// Contained Groups.
    groups: Vec<Group>,
}

/// Voters belong to Groups, and certain questions may be gated by Group membership.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Group {
    /// Group unique ID. Must be unique per election, i.e. not namespaced by Electorate.
    #[serde(rename = "_id")]
    id: Id,
    /// Friendly group name.
    name: String,
}

/// Example test data.
#[cfg(test)]
mod examples {
    use super::*;

    impl Electorate {
        pub fn example1() -> Self {
            Self {
                name: "Societies".to_string(),
                groups: vec![],
            }
        }

        pub fn example2() -> Self {
            Self {
                name: "Courses".to_string(),
                groups: vec![],
            }
        }
    }
}
