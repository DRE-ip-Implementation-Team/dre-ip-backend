use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::election_core::ElectionCore;

/// An election from the database, with its unique ID.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Election {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    election: ElectionCore,
}

impl Deref for Election {
    type Target = ElectionCore;

    fn deref(&self) -> &Self::Target {
        &self.election
    }
}

impl DbEntity for Election {
    fn id(&self) -> Id {
        self.id
    }
}
