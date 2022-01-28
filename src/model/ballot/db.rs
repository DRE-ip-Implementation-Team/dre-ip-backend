use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::bson::Id;

use super::Ballot;

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct DbBallot {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    ballot: Ballot,
}

impl Deref for DbBallot {
    type Target = Ballot;

    fn deref(&self) -> &Self::Target {
        &self.ballot
    }
}
