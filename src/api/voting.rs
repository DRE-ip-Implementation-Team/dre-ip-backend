use mongodb::bson::doc;
use rocket::{http::Status, serde::json::Json, Route};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    ballot::{Audited, Ballot, Confirmed, Receipt, Signature, Unconfirmed, UNCONFIRMED},
    election::Election,
    mongodb::{Coll, Id},
    voter::Voter,
};

use super::common::{active_election_by_id, voter_by_token};

pub fn routes() -> Vec<Route> {
    routes![get_confirmed, cast_ballots, audit_ballots, confirm_ballots,]
}

// TODO: ensure everything is concurrency-safe with transactions.

#[get("/voter/elections/<election_id>/questions/confirmed")]
async fn get_confirmed(
    token: AuthToken<Voter>,
    election_id: Id,
    voters: Coll<Voter>,
) -> Result<Json<Vec<Id>>> {
    // Get the voter.
    let voter = voter_by_token(&token, &voters).await?;

    // Find what they've voted for.
    let confirmed = voter
        .election_voted
        .get(&election_id)
        .cloned()
        .unwrap_or_else(|| Vec::new());

    Ok(Json(confirmed))
}

#[post(
    "/voter/elections/<election_id>/votes/cast",
    data = "<ballot_specs>",
    format = "json"
)]
async fn cast_ballots(
    _token: AuthToken<Voter>,
    election_id: Id,
    ballot_specs: Json<Vec<BallotSpec>>,
    elections: Coll<Election>,
    ballots: Coll<Ballot<Unconfirmed>>,
) -> Result<Json<Vec<Receipt<Unconfirmed>>>> {
    // Get the election.
    let election = active_election_by_id(election_id, &elections).await?;

    // Ensure that the questions and candidates exist.
    for ballot_spec in ballot_specs.0.iter() {
        if let Some(question) = election.questions.get(&ballot_spec.question) {
            if question.candidate(&ballot_spec.candidate).is_none() {
                return Err(Error::Status(
                    Status::NotFound,
                    format!(
                        "Candidate '{}' not found for question '{:?}'",
                        ballot_spec.candidate, ballot_spec.question
                    ),
                ));
            }
        } else {
            return Err(Error::Status(
                Status::NotFound,
                format!("Question '{:?}' not found", ballot_spec.question),
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
            let question = election.questions.get(&ballot_spec.question).unwrap(); // Already checked.
            let yes_candidate = ballot_spec.candidate; // Already checked that it exists.
            let no_candidates = question
                .candidates
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
            )
            .ok_or_else(|| {
                Error::Status(
                    Status::InternalServerError,
                    format!("Duplicate candidates for question {:?}", question.id),
                )
            })?;
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

#[post(
    "/voter/elections/<election_id>/votes/audit",
    data = "<ballot_recalls>",
    format = "json"
)]
async fn audit_ballots(
    _token: AuthToken<Voter>,
    election_id: Id,
    ballot_recalls: Json<Vec<BallotRecall>>,
    elections: Coll<Election>,
    unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
    audited_ballots: Coll<Ballot<Audited>>,
) -> Result<Json<Vec<Receipt<Audited>>>> {
    // Get the election and ballots.
    let election = active_election_by_id(election_id, &elections).await?;
    let recalled_ballots =
        recall_ballots(ballot_recalls.0, &unconfirmed_ballots, &election).await?;

    // Update ballots in DB.
    let mut new_ballots = Vec::with_capacity(recalled_ballots.len());
    for ballot in recalled_ballots {
        let audited = ballot.audit();
        let filter = doc! {
            "_id": *audited.id,
            "election_id": *election_id,
            "question_id": *audited.question_id,
            "state": UNCONFIRMED,
        };
        let result = audited_ballots.replace_one(filter, &audited, None).await?;
        // Sanity check.
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.modified_count, 1);
        new_ballots.push(audited);
    }

    // Return receipts.
    let receipts = new_ballots
        .into_iter()
        .map(|ballot| Receipt::from_ballot(ballot, &election))
        .collect();

    Ok(Json(receipts))
}

#[post(
    "/voter/elections/<election_id>/votes/confirm",
    data = "<ballot_recalls>",
    format = "json"
)]
async fn confirm_ballots(
    token: AuthToken<Voter>,
    election_id: Id,
    ballot_recalls: Json<Vec<BallotRecall>>,
    voters: Coll<Voter>,
    elections: Coll<Election>,
    unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
    confirmed_ballots: Coll<Ballot<Confirmed>>,
) -> Result<Json<Vec<Receipt<Confirmed>>>> {
    // Get the election and ballots.
    let mut election = active_election_by_id(election_id, &elections).await?;
    let recalled_ballots =
        recall_ballots(ballot_recalls.0, &unconfirmed_ballots, &election).await?;

    // Update DB.
    let mut new_ballots = Vec::with_capacity(recalled_ballots.len());
    for ballot in recalled_ballots {
        // Check that the user has not already voted on these questions.
        let filter = doc! {
            "_id": *token.id(),
            "election_voted": {
                election_id: {
                    "$nin": [*ballot.question_id],
                },
            },
        };
        let update = doc! {
            "$push": {
                "election_voted": {
                    election_id: [*ballot.question_id],
                },
            },
        };
        let result = voters.update_one(filter, update, None).await?;
        if result.modified_count != 1 {
            return Err(Error::Status(
                Status::BadRequest,
                format!(
                    "Voter {:?} does not exist or has already voted on {:?}",
                    token.id(),
                    ballot.question_id
                ),
            ));
        }

        // TODO refresh election and initiate transaction.
        // TODO is there a better way to store candidate totals? This feels dodgy even if we add transactions.

        // Confirm ballot.
        let mut totals = election.question_totals(ballot.question_id).unwrap(); // A question must exist to cast a ballot on it in the first place.
        let confirmed = ballot.confirm(&mut totals);
        let filter = doc! {
            "_id": *confirmed.id,
            "election_id": *election_id,
            "question_id": *confirmed.question_id,
            "state": UNCONFIRMED,
        };
        let result = confirmed_ballots
            .replace_one(filter, &confirmed, None)
            .await?;
        // Sanity check.
        assert_eq!(result.matched_count, 1);
        assert_eq!(result.modified_count, 1);

        // Write updated candidate totals.
        elections
            .replace_one(doc! {"_id": *election_id}, &election, None)
            .await?;

        new_ballots.push(confirmed);
    }

    // Return receipts.
    let receipts = new_ballots
        .into_iter()
        .map(|ballot| Receipt::from_ballot(ballot, &election))
        .collect();

    Ok(Json(receipts))
}

/// Get the given unconfirmed ballots, verifying their signatures.
async fn recall_ballots(
    ballot_recalls: Vec<BallotRecall>,
    unconfirmed_ballots: &Coll<Ballot<Unconfirmed>>,
    election: &Election,
) -> Result<Vec<Ballot<Unconfirmed>>> {
    let mut ballots = Vec::with_capacity(ballot_recalls.len());
    for recall in ballot_recalls {
        let filter = doc! {
            "_id": *recall.ballot_id,
            "election_id": *election.id,
            "question_id": *recall.question_id,
            "state": UNCONFIRMED,
        };
        let ballot = unconfirmed_ballots
            .find_one(filter, None)
            .await?
            .and_then(|ballot| {
                // Verify ownership of the ballot. If this fails, we return
                // an error indistinguishable from the ballot ID not existing,
                // so an attacker cannot learn anything about valid ballot IDs.
                let true_signature = Receipt::from_ballot(ballot.clone(), &election).signature;
                if true_signature == recall.signature {
                    Some(ballot)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                Error::Status(
                    Status::NotFound,
                    format!("Ballot not found with ID {:?}", recall.ballot_id),
                )
            })?;
        ballots.push(ballot);
    }
    Ok(ballots)
}

/// A ballot that the user wishes to cast, representing a specific candidate
/// for a specific question.
#[derive(Debug, Deserialize, Serialize)]
struct BallotSpec {
    pub question: Id,
    pub candidate: String,
}

/// A ballot that the user wishes to recall in order to audit or confirm.
/// The ballot is identified by its ID and question ID, and ownership of this
/// ballot is verified by the signature, which only the owning voter will have.
#[derive(Debug, Deserialize, Serialize)]
struct BallotRecall {
    pub ballot_id: Id,
    pub question_id: Id,
    #[serde(with = "dre_ip::group::serde_bytestring")]
    pub signature: Signature,
}
