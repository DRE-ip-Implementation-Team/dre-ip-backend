use mongodb::{
    bson::{oid::ObjectId, DateTime},
    Collection,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Election {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
    finalised: bool,
    start_time: DateTime,
    end_time: DateTime,
    groups: Vec<Group>,
    questions: Vec<Question>,
    ballots: Vec<Ballot>,
}

impl Election {
    pub fn ballots(self) -> Vec<Ballot> {
        self.ballots
    }
}

pub type Elections = Collection<Election>;

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Group {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Question {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    description: String,
    group_constraints: Vec<ObjectId>,
    candidates: Vec<Candidate>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Candidate {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    name: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    election_id: Option<ObjectId>,
    r: String,
    z: usize,
    p: String,
    #[serde(flatten)]
    action: Action,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Action {
    Confirmed,
    Audited { r: String, v: usize },
}
