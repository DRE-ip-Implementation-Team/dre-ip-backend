use serde::{Deserialize, Serialize};

pub mod db;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct Ballot {
    votes: Vec<Vote>,
    pwf: OneYesPwf,
    #[serde(flatten)]
    state: State,
}

impl Ballot {
    pub fn new(votes: Vec<Vote>, pwf: OneYesPwf, state: State) -> Self {
        Self { votes, pwf, state }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    R: String,
    Z: usize,
    pwf: ValuePwf,
    #[serde(flatten)]
    secret: Option<Secret>,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Secret {
    r: (),
    v: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
/// ZKP that only one vote is 1
pub struct OneYesPwf {
    a: (),
    b: (),
    r: (),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
/// ZKP that vote value is 0 or 1
pub struct ValuePwf {
    c1: (),
    c2: (),
    r1: (),
    r2: (),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Unconfirmed,
    Audited,
    Confirmed,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Secrets {
    r: String,
    v: usize,
}
