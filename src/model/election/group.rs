use serde::{Deserialize, Serialize};

use crate::model::mongodb::bson::Id;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, FromForm)]
#[serde(rename = "camelCase")]
pub struct Group {
    #[serde(rename = "_id")]
    id: Id,
    name: String,
}
