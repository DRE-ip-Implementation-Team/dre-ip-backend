use mongodb::{
    bson::{doc, oid::ObjectId},
    Collection,
};
use serde::{Deserialize, Serialize};

use self::db::DbVoter;

use super::{
    auth::token::{Privileges, User},
    sms::Sms,
};

pub mod db;

#[derive(Serialize, Deserialize)]
pub struct Voter {
    sms: Sms,
}

impl Voter {
    pub fn new(sms: Sms) -> Self {
        Self { sms }
    }

    pub fn into_db_voter(self, id: ObjectId) -> DbVoter {
        DbVoter::new(id, self)
    }
}

impl User for Voter {
    fn privileges() -> Privileges {
        Privileges::Voter
    }
}

pub type PutVoters = Collection<Voter>;
