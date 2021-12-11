use mongodb::{bson::doc, Collection};
use serde::{Deserialize, Serialize};

use super::{
    auth::token::{Rights, User},
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
}

impl User for Voter {
    fn rights() -> Rights {
        Rights::Voter
    }
}

pub type PutVoters = Collection<Voter>;
