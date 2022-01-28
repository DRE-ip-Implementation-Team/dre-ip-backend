use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{bson::Id, entity::DbEntity};

use super::Election;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbElection {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    election: Election,
}

impl DbElection {
    pub fn new(id: Id, election: Election) -> Self {
        Self { id, election }
    }
}

impl Deref for DbElection {
    type Target = Election;

    fn deref(&self) -> &Self::Target {
        &self.election
    }
}

impl DbEntity for DbElection {
    fn id(&self) -> Id {
        self.id
    }
}
