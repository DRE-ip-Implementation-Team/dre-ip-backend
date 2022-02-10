use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::election_core::ElectionCore;

/// An election from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Election {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub election: ElectionCore,
}

impl Deref for Election {
    type Target = ElectionCore;

    fn deref(&self) -> &Self::Target {
        &self.election
    }
}

impl DerefMut for Election {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.election
    }
}

impl DbEntity for Election {
    fn id(&self) -> Id {
        self.id
    }
}
