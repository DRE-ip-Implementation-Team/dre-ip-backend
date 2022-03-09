use std::ops::{Deref, DerefMut};

use dre_ip::NoSecrets;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::model::{election::ElectionCore, mongodb::Id};

/// An election from the database, with its unique ID.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(serialize = "S: Serialize", deserialize = "for<'a> S: Deserialize<'a>"))]
pub struct Election<S> {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub election: ElectionCore<S>,
}

impl<S> Election<S> {
    /// Erase the secrets from this election.
    pub fn erase_secrets(self) -> Election<NoSecrets> {
        Election {
            id: self.id,
            election: self.election.erase_secrets(),
        }
    }
}

impl<S> Deref for Election<S> {
    type Target = ElectionCore<S>;

    fn deref(&self) -> &Self::Target {
        &self.election
    }
}

impl<S> DerefMut for Election<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.election
    }
}
