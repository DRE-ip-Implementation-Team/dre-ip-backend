use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Serialize};

use crate::model::mongodb::Id;

use super::voter_core::VoterCore;

/// A voter user from the database, with its unique ID.
#[derive(Debug, Serialize, Deserialize)]
pub struct Voter {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub voter: VoterCore,
}

impl Deref for Voter {
    type Target = VoterCore;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

impl DerefMut for Voter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.voter
    }
}
