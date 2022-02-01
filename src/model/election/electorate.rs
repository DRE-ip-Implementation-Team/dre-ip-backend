use serde::{Deserialize, Serialize};

use super::Group;

#[derive(Debug, Serialize, Deserialize)]
pub struct Electorate {
    name: String,
    groups: Vec<Group>,
}
