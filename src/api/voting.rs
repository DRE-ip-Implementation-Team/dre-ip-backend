use std::collections::HashMap;

use mongodb::{bson::doc, Client};
use rocket::{futures::TryStreamExt, http::Status, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    ballot::{Audited, Ballot, Confirmed, Receipt, Signature, Unconfirmed, UNCONFIRMED},
    candidate_totals::CandidateTotals,
    election::Election,
    mongodb::{Coll, Id},
    voter::Voter,
};

use super::common::{active_election_by_id, voter_by_token};

pub fn routes() -> Vec<Route> {
    routes![get_allowed, cast_ballots, audit_ballots, confirm_ballots]
}

#[get("/voter/elections/<election_id>/questions/allowed")]
async fn get_allowed(
    token: AuthToken<Voter>,
    election_id: Id,
    voters: Coll<Voter>,
) -> Result<Json<Vec<Id>>> {
    // Get the voter.
    let voter = voter_by_token(&token, &voters).await?;

    // Find what questions they can still vote for.
    let allowed = voter
        .allowed_questions
        .get(&election_id)
        .cloned()
        .unwrap_or_else(Vec::new);

    Ok(Json(allowed))
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
    db_client: &State<Client>,
) -> Result<Json<Vec<Receipt<Unconfirmed>>>> {
    // Get the election.
    let election = active_election_by_id(election_id, &elections).await?;

    // Ensure that the questions and candidates exist.
    for ballot_spec in &*ballot_specs {
        if let Some(question) = election.questions.get(&ballot_spec.question) {
            if !question.candidates.contains(&ballot_spec.candidate) {
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
                .filter(|name| name != &&yes_candidate)
                .cloned()
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

    // Insert ballots into DB within a transaction, so this entire endpoint is atomic.
    {
        let mut session = db_client.start_session(None).await?;
        session.start_transaction(None).await?;
        ballots
            .insert_many_with_session(new_ballots.iter(), None, &mut session)
            .await?;
        session.commit_transaction().await?;
    }

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
    db_client: &State<Client>,
) -> Result<Json<Vec<Receipt<Audited>>>> {
    // Get the election and ballots.
    let election = active_election_by_id(election_id, &elections).await?;
    let recalled_ballots =
        recall_ballots(ballot_recalls.0, &unconfirmed_ballots, &election).await?;

    // Update ballots in DB using a transaction so the whole endpoint is atomic.
    let mut new_ballots = Vec::with_capacity(recalled_ballots.len());
    {
        let mut session = db_client.start_session(None).await?;
        session.start_transaction(None).await?;

        for ballot in recalled_ballots {
            let audited = ballot.audit();
            let filter = doc! {
                "_id": *audited.id,
                "election_id": *election_id,
                "question_id": *audited.question_id,
                "state": UNCONFIRMED,
            };
            let result = audited_ballots
                .replace_one_with_session(filter, &audited, None, &mut session)
                .await?;
            assert_eq!(result.modified_count, 1);
            new_ballots.push(audited);
        }

        session.commit_transaction().await?;
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
#[allow(clippy::too_many_arguments)]
async fn confirm_ballots(
    token: AuthToken<Voter>,
    election_id: Id,
    ballot_recalls: Json<Vec<BallotRecall>>,
    voters: Coll<Voter>,
    elections: Coll<Election>,
    unconfirmed_ballots: Coll<Ballot<Unconfirmed>>,
    confirmed_ballots: Coll<Ballot<Confirmed>>,
    candidate_totals: Coll<CandidateTotals>,
    db_client: &State<Client>,
) -> Result<Json<Vec<Receipt<Confirmed>>>> {
    // Get the election and ballots.
    let election = active_election_by_id(election_id, &elections).await?;
    let recalled_ballots =
        recall_ballots(ballot_recalls.0, &unconfirmed_ballots, &election).await?;

    // Update DB in a transaction so the whole endpoint is atomic.
    let mut new_ballots = Vec::with_capacity(recalled_ballots.len());
    {
        let mut session = db_client.start_session(None).await?;
        session.start_transaction(None).await?;

        for ballot in recalled_ballots {
            // Check that the user is eligible to vote on this question.
            let filter = doc! {
                "_id": *token.id,
                "allowed_questions": {
                    election_id: {
                        "$in": [*ballot.question_id],
                    },
                },
            };
            let update = doc! {
                "$pull": {
                    "election_voted": {
                        election_id: {
                            "$eq": *ballot.question_id,
                        },
                    },
                },
            };
            let result = voters
                .update_one_with_session(filter, update, None, &mut session)
                .await?;
            if result.modified_count != 1 {
                return Err(Error::Status(
                    Status::BadRequest,
                    format!(
                        "Voter {:?} does not exist or cannot vote on {:?}",
                        token.id, ballot.question_id
                    ),
                ));
            }

            // Get candidate totals.
            let filter = doc! {
                "election_id": *election_id,
                "question_id": *ballot.question_id,
            };
            let mut totals = candidate_totals
                .find(filter, None)
                .await?
                .try_collect::<Vec<_>>()
                .await?;
            assert_eq!(totals.len(), ballot.crypto.votes.len());
            // Convert to hashmap.
            let mut totals_map = totals
                .iter_mut()
                .map(|t| (t.candidate_name.clone(), &mut t.totals.totals))
                .collect::<HashMap<_, _>>();

            // Confirm ballot.
            let confirmed = ballot.confirm(&mut totals_map);
            let filter = doc! {
                "_id": *confirmed.id,
                "election_id": *election_id,
                "question_id": *confirmed.question_id,
                "state": UNCONFIRMED,
            };
            let result = confirmed_ballots
                .replace_one_with_session(filter, &confirmed, None, &mut session)
                .await?;
            assert_eq!(result.modified_count, 1);

            // Write updated candidate totals.
            for t in totals {
                let filter = doc! {
                    "election_id": *election_id,
                    "question_id": *confirmed.question_id,
                    "candidate_name": t.candidate_name.clone(),
                };
                let result = candidate_totals
                    .replace_one_with_session(filter, t, None, &mut session)
                    .await?;
                assert_eq!(result.modified_count, 1);
            }

            new_ballots.push(confirmed);
        }

        session.commit_transaction().await?;
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
                let true_signature = Receipt::from_ballot(ballot.clone(), election).signature;
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

#[cfg(test)]
mod tests {
    use backend_test::backend_test;
    use chrono::{Duration, Utc};
    use dre_ip::ElectionResults;
    use dre_ip::group::{DreipScalar, DreipPublicKey};
    use mongodb::Database;
    use rocket::{futures::{StreamExt, TryStreamExt}, http::ContentType, local::asynchronous::Client, serde::json::serde_json};

    use crate::model::{
        election::{Election, ElectionSpec, NewElection, QuestionSpec},
        sms::Sms,
    };

    use super::*;

    /// Insert test data, returning the election ID and the ID of the allowed question,
    /// which may differ between runs.
    async fn insert_test_data(db: &Database) -> (Id, Id) {
        // Create some elections, only one of which is active.
        let election: NewElection = ElectionSpec::finalised_example().into();
        let election1 = Election {
            id: Id::new(),
            election,
        };
        let election: NewElection = ElectionSpec::unfinalised_example().into();
        let election2 = Election {
            id: Id::new(),
            election,
        };
        let elections = Coll::<Election>::from_db(db);
        elections.insert_many(vec![&election1, &election2], None).await.unwrap();

        // Allow the voter to vote on one of the two questions.
        let voters = Coll::<Voter>::from_db(db);
        let mut voter = voters
            .find_one(
                doc! {
                    "sms": Sms::example(),
                },
                None,
            )
            .await
            .unwrap()
            .unwrap();
        let allowed_question = *election1
            .questions
            .iter()
            .find_map(|(id, question)| {
                if question.description == QuestionSpec::example1().description {
                    Some(id)
                } else {
                    None
                }
            })
            .unwrap();
        voter
            .allowed_questions
            .insert(election1.id, vec![allowed_question]);
        voters
            .replace_one(
                doc! {
                    "_id": *voter.id,
                },
                &voter,
                None,
            )
            .await
            .unwrap();
        (election1.id, allowed_question)
    }

    /// Dump the current state of the database to stdout; useful for debugging.
    async fn dump_db_state(db: &Database) {
        println!("\nVoters:");
        let mut voters = Coll::<Voter>::from_db(db).find(None, None).await.unwrap();
        while let Some(Ok(voter)) = voters.next().await {
            println!("{:?}", voter);
        }

        println!("\nElections:");
        let mut elections = Coll::<Election>::from_db(db).find(None, None).await.unwrap();
        while let Some(Ok(election)) = elections.next().await {
            println!("{:?}", election);
        }

        println!("\nUnconfirmed Ballots:");
        let mut ballots = Coll::<Ballot<Unconfirmed>>::from_db(db).find(doc!{"state": "Unconfirmed"}, None).await.unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }
        println!("\nAudited Ballots:");
        let mut ballots = Coll::<Ballot<Audited>>::from_db(db).find(doc!{"state": "Audited"}, None).await.unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }
        println!("\nConfirmed Ballots:");
        let mut ballots = Coll::<Ballot<Confirmed>>::from_db(db).find(doc!{"state": "Confirmed"}, None).await.unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }

        println!("\nCandidate Totals:");
        let mut totals = Coll::<CandidateTotals>::from_db(db).find(None, None).await.unwrap();
        println!("yeet");
        while let Some(Ok(total)) = totals.next().await {
            println!("{:?}", total);
        }
    }

    #[backend_test(voter)]
    async fn get_allowed(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&db).await;

        // Get the allowed questions.
        let response = client.get(uri!(get_allowed(election_id))).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        // Ensure they are correct.
        let raw_response = response.into_string().await.unwrap();
        let allowed: Vec<Id> = serde_json::from_str(&raw_response).unwrap();
        let expected = vec![question_id];
        assert_eq!(allowed, expected);
    }

    #[backend_test(voter)]
    async fn cast_ballots(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&db).await;

        // Vote on the question we are allowed to.
        let candidate_id = "Chris Riches".to_string();
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: candidate_id.clone(),
        }];

        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        // Ensure the response is a valid receipt.
        let raw_response = response.into_string().await.unwrap();
        let receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response).unwrap().into_iter().next().unwrap();

        // Ensure the receipt passes validation.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc!{"_id": *election_id}, None).await.unwrap().unwrap();
        // Validate PWFs.
        assert!(receipt.crypto.verify(&election.crypto, receipt.id.to_bytes()).is_ok());
        // Validate signature.
        let mut msg = receipt.crypto.to_bytes();
        msg.extend(receipt.id.to_bytes());
        msg.extend(receipt.state.as_ref());
        assert!(election.crypto.public_key.verify(&msg, &receipt.signature));

        // Ensure the ballot in the database is correct.
        let ballot = Coll::<Ballot<Unconfirmed>>::from_db(&db)
            .find_one(doc!{"_id": *receipt.id}, None).await.unwrap().unwrap();
        // Check metadata.
        assert_eq!(ballot.election_id, election_id);
        assert_eq!(ballot.question_id, question_id);
        assert!(ballot.creation_time < Utc::now() && ballot.creation_time + Duration::minutes(1) > Utc::now());
        let mut yes_votes = 0;
        for (candidate, vote) in ballot.crypto.votes.iter() {
            // Check the vote value is correct.
            if candidate == &candidate_id {
                assert_eq!(vote.v, DreipScalar::one());
                yes_votes += 1;
            } else {
                assert_eq!(vote.v, DreipScalar::zero());
            }
            // Check the receipt matches the internal vote.
            let receipt_vote = receipt.crypto.votes.get(candidate).unwrap();
            assert_eq!(vote.R, receipt_vote.R);
            assert_eq!(vote.Z, receipt_vote.Z);
            assert_eq!(vote.pwf, receipt_vote.pwf);
            // Check the public values were correctly calculated from the secrets.
            assert_eq!(vote.R, election.crypto.g2 * vote.r);
            assert_eq!(vote.Z, election.crypto.g1 * (vote.r + vote.v));
        }
        // Check there was exactly one yes vote (already guaranteed by the PWF, but sanity checks are good).
        assert_eq!(yes_votes, 1);
    }

    #[backend_test(voter)]
    async fn audit(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&db).await;

        // Vote on the question we are allowed to.
        let candidate_id = "Chris Riches".to_string();
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: candidate_id.clone(),
        }];

        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());
        let raw_response = response.into_string().await.unwrap();
        let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response).unwrap().into_iter().next().unwrap();

        // Audit the ballot.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.id,
            question_id,
            signature: first_receipt.signature,
        }];
        let response = client
            .post(uri!(audit_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        // Ensure the response is a valid receipt.
        let raw_response = response.into_string().await.unwrap();
        let second_receipt: Receipt<Audited> = serde_json::from_str::<Vec<_>>(&raw_response).unwrap().into_iter().next().unwrap();

        // Ensure the receipts match.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc!{"_id": *election_id}, None).await.unwrap().unwrap();
        assert_eq!(first_receipt.id, second_receipt.id);
        assert_eq!(first_receipt.crypto.pwf, second_receipt.crypto.pwf);
        for (candidate, vote1) in first_receipt.crypto.votes.iter() {
            let vote2 = second_receipt.crypto.votes.get(candidate).unwrap();
            assert_eq!(vote1.pwf, vote2.pwf);
            assert_eq!(vote1.R, vote2.R);
            assert_eq!(vote1.Z, vote2.Z);
            assert_eq!(vote1.R, election.crypto.g2 * vote2.r);
            assert_eq!(vote1.Z, election.crypto.g1 * (vote2.r + vote2.v));
        }

        // Validate PWFs.
        assert!(second_receipt.crypto.verify(&election.crypto, second_receipt.id.to_bytes()).is_ok());
        // Validate signature.
        let mut msg = second_receipt.crypto.to_bytes();
        msg.extend(second_receipt.id.to_bytes());
        msg.extend(second_receipt.state.as_ref());
        assert!(election.crypto.public_key.verify(&msg, &second_receipt.signature));

        // Check the candidate totals weren't affected.
        let totals: Vec<CandidateTotals> = Coll::<CandidateTotals>::from_db(&db)
            .find(None, None).await.unwrap().try_collect().await.unwrap();
        for total in totals.iter() {
            assert_eq!(total.totals.totals.r_sum, DreipScalar::zero());
            assert_eq!(total.totals.totals.tally, DreipScalar::zero());
        }
    }

    #[backend_test(voter)]
    async fn confirm(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&db).await;

        // Vote on the question we are allowed to.
        let candidate_id = "Chris Riches".to_string();
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: candidate_id.clone(),
        }];

        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());
        let raw_response = response.into_string().await.unwrap();
        let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response).unwrap().into_iter().next().unwrap();

        dump_db_state(&db).await;

        // Confirm the ballot.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.id,
            question_id,
            signature: first_receipt.signature,
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        // Ensure the response is a valid receipt.
        let raw_response = response.into_string().await.unwrap();
        let second_receipt: Receipt<Confirmed> = serde_json::from_str::<Vec<_>>(&raw_response).unwrap().into_iter().next().unwrap();

        // Ensure the receipts match.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc!{"_id": *election_id}, None).await.unwrap().unwrap();
        assert_eq!(first_receipt.id, second_receipt.id);
        assert_eq!(first_receipt.crypto, second_receipt.crypto);

        // Validate PWFs.
        assert!(second_receipt.crypto.verify(&election.crypto, second_receipt.id.to_bytes()).is_ok());
        // Validate signature.
        let mut msg = second_receipt.crypto.to_bytes();
        msg.extend(second_receipt.id.to_bytes());
        msg.extend(second_receipt.state.as_ref());
        assert!(election.crypto.public_key.verify(&msg, &second_receipt.signature));

        // Check the candidate totals are correct.
        let candidate_totals: Vec<CandidateTotals> = Coll::<CandidateTotals>::from_db(&db)
            .find(None, None).await.unwrap().try_collect().await.unwrap();
        for total in candidate_totals.iter() {
            if total.question_id == question_id && total.candidate_name == candidate_id {
                assert_eq!(total.totals.totals.tally, DreipScalar::one());
            } else {
                assert_eq!(total.totals.totals.tally, DreipScalar::zero());
            }
        }
        let mut ballots = HashMap::new();
        ballots.insert(second_receipt.id.to_bytes(), second_receipt.crypto);
        let mut totals = HashMap::new();
        for total in candidate_totals {
            totals.insert(total.candidate_name.clone(), total.totals.totals);
        }
        let results = ElectionResults {
            election: election.election.crypto,
            ballots,
            totals,
        };
        assert!(results.verify().is_ok());
    }

    #[backend_test(voter)]
    async fn bad_casts(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&db).await;

        // Try voting on a non-existent question.
        let ballot_specs = vec![BallotSpec {
            question: Id::new(),
            candidate: "John Smith".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try voting on an allowed question but for a non-existent candidate.
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: "Nobody".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try voting on an inactive election.
        let inactive_election = Coll::<Election>::from_db(&db)
            .find_one(doc!{"finalised": false}, None).await.unwrap().unwrap();
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: "Chris Riches".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(inactive_election.id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Ensure nothing we did had any effect.
        let ballots = Coll::<Ballot<Unconfirmed>>::from_db(&db)
            .find(None, None).await.unwrap();
        assert_eq!(ballots.try_collect::<Vec<_>>().await.unwrap().len(), 0);
    }

    #[backend_test(voter)]
    async fn bad_audit_confirms() {
        // TODO
    }
}
