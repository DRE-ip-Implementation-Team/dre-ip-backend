use serde::{Deserialize, Serialize};

/// Core ballot data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct BallotCore {
    /// The individual yes/no vote for each candidate within this ballot.
    votes: Vec<Vote>,
    /// The proof of well-formedness that exactly one of the votes is "yes".
    pwf: BallotProof,
    /// The current state of the ballot.
    #[serde(flatten)]
    state: BallotState,
}

/// A single yes/no vote for a single candidate, as stored in the database.
#[allow(non_snake_case)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    /// The public random value R.
    R: Vec<u8>,
    /// The public vote value Z.
    Z: Vec<u8>,
    /// The proof of well-formedness that the secret vote value is 0 or 1.
    pwf: VoteProof,
    /// Secret vote values, only present for non-confirmed ballots.
    #[serde(flatten)]
    secret: Option<VoteSecrets>,
}

/// Secret vote values.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteSecrets {
    /// The secret random value r.
    r: Vec<u8>,
    /// The secret vote value v.
    v: Vec<u8>,
}

/// Ballot proof of well-formedness as per DRE-ip protocol.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallotProof {
    /// Proof value a.
    a: Vec<u8>,
    /// Proof value b.
    b: Vec<u8>,
    /// Proof response value.
    r: Vec<u8>,
}

/// Vote proof of well-formedness as per DRE-ip protocol.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteProof {
    /// Challenge value one.
    c1: Vec<u8>,
    /// Challenge value two.
    c2: Vec<u8>,
    /// Response value one.
    r1: Vec<u8>,
    /// Response value two.
    r2: Vec<u8>,
}

/// Ballot state: has it been confirmed or audited yet?
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BallotState {
    Unconfirmed,
    Audited,
    Confirmed,
}
