use rocket::{
    http::Status,
    Route,
    serde::json::Json,
};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    ballot::{Audited, Ballot, Confirmed, Receipt, Unconfirmed},
    election::Election,
    mongodb::{Coll, Id},
    voter::Voter,
};

use super::common::{active_election_by_id, voter_by_token};

pub fn routes() -> Vec<Route> {
    routes![
        get_confirmed,
        cast_ballots,
        audit_ballots,
        confirm_ballots,
    ]
}

#[get("/voter/elections/<election_id>/questions/confirmed")]
async fn get_confirmed(token: AuthToken<Voter>, election_id: Id,
                       voters: Coll<Voter>) -> Result<Json<Vec<Id>>> {
    // Get the voter.
    let voter = voter_by_token(&token, &voters).await?;

    // Find what they've voted for.
    let confirmed = voter.election_voted
        .get(&election_id)
        .cloned()
        .unwrap_or_else(|| Vec::new());

    Ok(Json(confirmed))
}

#[post("/voter/elections/<election_id>/votes/cast", data = "<ballot_specs>", format = "json")]
async fn cast_ballots(_token: AuthToken<Voter>, election_id: Id,
                      ballot_specs: Json<Vec<BallotSpec>>,
                      elections: Coll<Election>, ballots: Coll<Ballot<Unconfirmed>>)
                      -> Result<Json<Vec<Receipt<Unconfirmed>>>> {
    // Get the election.
    let election = active_election_by_id(election_id, &elections).await?;

    // Ensure that the questions and candidates exist.
    for ballot_spec in ballot_specs.0.iter() {
        if let Some(question) = election.question(ballot_spec.question) {
            if question.candidate(&ballot_spec.candidate).is_none() {
                return Err(Error::Status(
                    Status::NotFound,
                    format!("Candidate '{}' not found for question '{:?}'",
                            ballot_spec.candidate, ballot_spec.question)
                ));
            }
        } else {
            return Err(Error::Status(
                Status::NotFound,
                format!("Question '{:?}' not found", ballot_spec.question)
            ));
        }
    }

    // Generate cryptographic ballots.
    // The scoped block is needed to force `rng` to be dropped before the next `await`.
    let mut new_ballots = Vec::new();
    {
        let mut rng = rand::thread_rng();
        for ballot_spec in ballot_specs.0 {
            // Get the yes and no candidates for this ballot.
            let question = election.question(ballot_spec.question).unwrap(); // Already checked.
            let yes_candidate = ballot_spec.candidate; // Already checked that it exists.
            let no_candidates = question.candidates
                .iter()
                .map(|c| c.name.clone())
                .filter(|name| name != &yes_candidate)
                .collect::<Vec<_>>();
            // Sanity check.
            assert_eq!(question.candidates.len() - 1, no_candidates.len());

            // Create the ballot.
            let ballot = Ballot::new(
                election_id,
                question.id,
                yes_candidate,
                no_candidates,
                &election,
                &mut rng,
            ).ok_or_else(|| Error::Status(
                Status::InternalServerError,
                format!("Duplicate candidates for question {:?}", question.id),
            ))?;
            new_ballots.push(ballot);
        }
    }

    // Insert ballots into DB.
    // TODO Ensure they expire if not audited or confirmed.
    ballots.insert_many(new_ballots.iter(), None).await?;

    // Return receipts.
    let receipts = new_ballots
        .into_iter()
        .map(|ballot| Receipt::from_ballot(ballot, &election))
        .collect();

    Ok(Json(receipts))
}

#[post("/voter/elections/<election_id>/votes/audit", data = "<ballots>", format = "json")]
async fn audit_ballots(token: AuthToken<Voter>, election_id: Id,
                       ballots: Json<Vec<Id>>) -> Result<Json<Vec<Receipt<Audited>>>> {
    // TODO Get the voter, election, and ballots.

    // TODO Mark the ballots as audited.

    // TODO Return receipts.

    todo!()
}

#[post("/voter/elections/<election_id>/votes/confirm", data = "<ballots>", format = "json")]
async fn confirm_ballots(token: AuthToken<Voter>, election_id: Id,
                         ballots: Json<Vec<Id>>) -> Result<Json<Vec<Receipt<Confirmed>>>> {
    // TODO Get the voter, election, and ballots.

    // TODO Check that the user has not already voted on these questions.

    // TODO Record that the user has voted on these questions.

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
