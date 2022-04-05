use serde::{Deserialize, Serialize};

use crate::model::{api::receipt::Signature, mongodb::Id};

/// A ballot that the voter wishes to cast, representing a specific candidate
/// for a specific question.
#[derive(Debug, Serialize, Deserialize)]
pub struct BallotSpec {
    pub question: Id,
    pub candidate: String,
}

/// A ballot that the voter wishes to recall in order to audit or confirm.
/// The ballot is identified by its ID and question ID, and ownership of this
/// ballot is verified by the signature, which only the owning voter will have.
#[derive(Debug, Serialize, Deserialize)]
pub struct BallotRecall {
    pub ballot_id: Id,
    pub question_id: Id,
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub signature: Signature,
}
