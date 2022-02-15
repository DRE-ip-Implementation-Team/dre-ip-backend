use serde::{Deserialize, Serialize};

/// An electorate is a potentially mutually-exclusive set of logically related groups.
/// Voters belong to groups, and certain questions may be gated by group membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Electorate {
    /// Electorate name.
    pub name: String,
    /// Are the groups in this electorate mutually exclusive?
    pub is_mutex: bool,
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
                is_mutex: false,
                groups: vec!["Quidditch".to_string(), "Extreme Moongolf".to_string()],
            }
        }

        pub fn example2() -> Self {
            Self {
                name: "Year Group".to_string(),
                is_mutex: true,
                groups: vec![
                    "Year 1".to_string(),
                    "Year 2".to_string(),
                    "Year 3".to_string(),
                    "Year 4+".to_string(),
                    "Postgrad".to_string(),
                ],
            }
        }
    }
}
