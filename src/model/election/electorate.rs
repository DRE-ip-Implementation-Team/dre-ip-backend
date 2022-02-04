use serde::{Deserialize, Serialize};

use crate::model::mongodb::bson::Id;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Electorate {
    name: String,
    /// IDs refer to [`Group`]s
    groups: Vec<Id>,
}

#[cfg(test)]
mod examples {
    use super::*;

    impl Electorate {
        pub fn example1() -> Self {
            Self {
                name: "Quidditch".to_string(),
                groups: vec![],
            }
        }

        pub fn example2() -> Self {
            Self {
                name: "Netball".to_string(),
                groups: vec![],
            }
        }
    }
}
