use serde::{Deserialize, Serialize};

/// An electorate is a potentially mutually-exclusive set of logically related groups.
/// Voters belong to groups, and certain questions may be gated by group membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Electorate {
    /// Electorate name.
    pub name: String,
    /// Contained Groups.
    pub groups: Vec<String>,
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
