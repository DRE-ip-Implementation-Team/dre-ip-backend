use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{bson::Id, entity::DbEntity};

use super::Voter;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbVoter {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    voter: Voter,
}

impl DbVoter {
    pub fn new(id: Id, voter: Voter) -> Self {
        Self { id, voter }
    }
}

impl Deref for DbVoter {
    type Target = Voter;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

impl DbEntity for DbVoter {
    fn id(&self) -> Id {
        self.id
    }
}
