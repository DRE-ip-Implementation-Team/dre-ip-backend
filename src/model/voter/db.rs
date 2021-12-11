use std::ops::Deref;

use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};
use serde::{Deserialize, Serialize};

use super::Voter;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DbVoter {
    #[serde(rename = "_id")]
    id: ObjectId,
    #[serde(flatten)]
    voter: Voter,
}

impl DbVoter {
    pub fn new(id: ObjectId, voter: Voter) -> Self {
        Self { id, voter }
    }

    pub fn id(&self) -> ObjectId {
        self.id
    }
}

impl Deref for DbVoter {
    type Target = Voter;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

pub type GetVoters = Collection<DbVoter>;
