use std::ops::{Deref, DerefMut};

use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

use super::election_core::ElectionCore;

/// An election from the database, with its unique ID.
#[derive(Debug, Serialize, Deserialize)]
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
