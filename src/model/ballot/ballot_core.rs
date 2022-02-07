use serde::{Deserialize, Serialize};

/// Core ballot data, as stored in the database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename = "camelCase")]
pub struct BallotCore {
    /// The individual yes/no vote for each candidate within this ballot.
    pub votes: Vec<Vote>,
    /// The proof of well-formedness that exactly one of the votes is "yes".
    pub pwf: BallotProof,
    /// The current state of the ballot.
    #[serde(flatten)]
    pub state: BallotState,
}

impl BallotCore {
    /// Convert to bytes for signing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for vote in self.votes.iter() {
            bytes.extend(vote.to_bytes());
        }
        bytes.extend(self.pwf.to_bytes());

        bytes
    }
}

/// A single yes/no vote for a single candidate, as stored in the database.
#[allow(non_snake_case)]
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vote {
    /// The public random value R.
    pub R: Vec<u8>,
    /// The public vote value Z.
    pub Z: Vec<u8>,
    /// The proof of well-formedness that the secret vote value is 0 or 1.
    pub pwf: VoteProof,
    /// Secret vote values, only present for non-confirmed ballots.
    #[serde(flatten)]
    pub secrets: Option<VoteSecrets>,
}

impl Vote {
    /// Convert to bytes for signing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.R.iter());
        bytes.extend(self.Z.iter());
        bytes.extend(self.pwf.to_bytes());
        if let Some(ref secrets) = self.secrets {
            bytes.extend(secrets.r.iter());
            bytes.extend(secrets.v.iter());
        }

        bytes
    }
}

/// Secret vote values.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteSecrets {
    /// The secret random value r.
    pub r: Vec<u8>,
    /// The secret vote value v.
    pub v: Vec<u8>,
}

/// Ballot proof of well-formedness as per DRE-ip protocol.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BallotProof {
    /// Proof value a.
    pub a: Vec<u8>,
    /// Proof value b.
    pub b: Vec<u8>,
    /// Proof response value.
    pub r: Vec<u8>,
}

impl BallotProof {
    /// Convert to bytes for signing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.a.iter());
        bytes.extend(self.b.iter());
        bytes.extend(self.r.iter());

        bytes
    }
}

/// Vote proof of well-formedness as per DRE-ip protocol.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoteProof {
    /// Challenge value one.
    pub c1: Vec<u8>,
    /// Challenge value two.
    pub c2: Vec<u8>,
    /// Response value one.
    pub r1: Vec<u8>,
    /// Response value two.
    pub r2: Vec<u8>,
}

impl VoteProof {
    /// Convert to bytes for signing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.c1.iter());
        bytes.extend(self.c2.iter());
        bytes.extend(self.r1.iter());
        bytes.extend(self.r2.iter());

        bytes
    }
}

/// Ballot state: has it been confirmed or audited yet?
#[derive(Debug, PartialEq, Eq, Copy, Clone, Serialize, Deserialize)]
pub enum BallotState {
    Unconfirmed,
    Audited,
    Confirmed,
}
