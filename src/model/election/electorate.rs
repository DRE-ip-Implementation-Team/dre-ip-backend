use std::collections::HashSet;

use serde::{Deserialize, Serialize};

/// An electorate is a potentially mutually-exclusive set of logically related groups.
/// Voters belong to groups, and certain questions may be gated by group membership.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Electorate {
    /// Electorate name.
    pub name: String,
    /// Contained groups.
    pub groups: HashSet<String>,
    /// Whether the groups are mutually exclusive.
    pub is_mutex: bool,
}

/// Example test data.
#[cfg(test)]
mod examples {
    use super::*;

    impl Electorate {
        pub fn example1() -> Self {
            Self {
                name: "Societies".to_string(),
                groups: HashSet::from_iter(
                    vec!["Quidditch", "Moongolf", "CompSoc"]
                        .into_iter()
                        .map(String::from),
                ),
                is_mutex: false,
            }
        }

        pub fn example2() -> Self {
            Self {
                name: "Courses".to_string(),
                groups: HashSet::from_iter(
                    vec!["CompSci", "Maths", "Physics"]
                        .into_iter()
                        .map(String::from),
                ),
                is_mutex: true,
            }
        }
    }
}
