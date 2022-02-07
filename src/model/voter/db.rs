use std::ops::Deref;

use serde::{Deserialize, Serialize};

use crate::model::mongodb::{DbEntity, Id};

use super::voter_core::VoterCore;

/// A voter user from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Voter {
    #[serde(rename = "_id")]
    id: Id,
    #[serde(flatten)]
    voter: VoterCore,
}

impl Deref for Voter {
    type Target = VoterCore;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

impl DbEntity for Voter {
    fn id(&self) -> Id {
        self.id
    }
}
