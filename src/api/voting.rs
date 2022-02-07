use rocket::Route;
use rocket::serde::json::Json;
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::model::{
    auth::AuthToken,
    ballot::Receipt,
    election::Election,
    mongodb::{Coll, Id},
    voter::Voter,
};

use super::common::get_voter_from_token;

/// We implement our DRE-ip over the P-256 elliptic curve.
type Group = dre_ip::group::p256::NistP256;

pub fn routes() -> Vec<Route> {
    routes![
        get_confirmed,
        cast_ballots,
        audit_ballots,
        confirm_ballots,
    ]
}

#[get("/voter/elections/<election_id>/questions/confirmed")]
async fn get_confirmed(token: AuthToken<Voter>, election_id: Id) -> Result<Json<Vec<Id>>> {
    todo!()
}

#[post("/voter/elections/<election_id>/votes/cast", data = "<ballots>", format = "json")]
async fn cast_ballots(token: AuthToken<Voter>, election_id: Id,
                      ballots: Json<Vec<BallotSpec>>, voters: Coll<Voter>,
                      elections: Coll<Election>) -> Result<Json<Vec<Receipt>>> {
    // TODO Get the voter and election.
    let voter = get_voter_from_token(&token, &voters).await?;

    // TODO Ensure that the questions and candidates exist.

    // TODO Generate cryptographic ballots.

    // TODO Insert ballots into DB. Ensure they expire if not audited or confirmed.

    // TODO Return receipt and encrypted ballot IDs.

    todo!()
}

#[post("/voter/elections/<election_id>/votes/audit", data = "<ballots>", format = "json")]
async fn audit_ballots(token: AuthToken<Voter>, election_id: Id,
                       ballots: Json<Vec<Id>>) -> Result<Json<Vec<Receipt>>> {
    // TODO Get the voter, election, and ballots.

    // TODO Mark the ballots as audited.

    // TODO Return receipts.

    todo!()
}

#[post("/voter/elections/<election_id>/votes/confirm", data = "<ballots>", format = "json")]
async fn confirm_ballots(token: AuthToken<Voter>, election_id: Id,
                         ballots: Json<Vec<Id>>) -> Result<Json<Vec<Receipt>>> {
    // TODO Get the voter, election, and ballots.

    // TODO Mark the ballots as confirmed and erase the secrets.

    // TODO Return receipts.

    todo!()
}

/// A ballot that the user wishes to cast, representing a specific candidate
/// for a specific question.
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
struct BallotSpec {
    pub question: Id,
    pub candidate: String,
}
