use std::collections::{HashMap, HashSet};

use chrono::Utc;
use mongodb::{bson::doc, options::ReplaceOptions, Client};
use rocket::{futures::TryStreamExt, http::Status, serde::json::Json, Route, State};

use crate::error::{Error, Result};
use crate::model::{
    api::{
        auth::AuthToken,
        ballot::{BallotRecall, BallotSpec},
        receipt::Receipt,
    },
    common::{
        allowed_questions::AllowedQuestions,
        ballot::{Audited, Confirmed, Unconfirmed},
        election::ElectionState,
    },
    db::{
        ballot::{Ballot, NewBallot},
        candidate_totals::{CandidateTotals, NewCandidateTotals},
        election::Election,
        voter::Voter,
    },
    mongodb::{Coll, Counter, Id},
};

pub fn routes() -> Vec<Route> {
    routes![
        has_joined,
        join_election,
        get_allowed,
        cast_ballots,
        audit_ballots,
        confirm_ballots
    ]
}

#[get("/elections/<election_id>/join")]
async fn has_joined(
    token: AuthToken<Voter>,
    election_id: Id,
    voters: Coll<Voter>,
) -> Result<Json<bool>> {
    let voter = voter_by_id(token.id, &voters).await?;
    Ok(Json(voter.allowed_questions.contains_key(&election_id)))
}

#[post("/elections/<election_id>/join", data = "<joins>", format = "json")]
async fn join_election(
    token: AuthToken<Voter>,
    election_id: Id,
    joins: Json<HashMap<String, HashSet<String>>>,
    elections: Coll<Election>,
    voters: Coll<Voter>,
) -> Result<()> {
    let voter = voter_by_id(token.id, &voters).await?;
    // Reject if voter has already joined the election
    if voter.allowed_questions.contains_key(&election_id) {
        return Err(Error::Status(
            Status::Forbidden,
            format!(
                "Voter has already joined election with ID '{}'",
                election_id
            ),
        ));
    }

    let election = active_election_by_id(election_id, &elections).await?;

    // Check that electorates and groups exist and meet mutex requirements
    for (electorate_name, groups) in &joins.0 {
        let electorate = election.electorates.get(electorate_name).ok_or_else(|| {
            Error::not_found(format!("Electorate with name '{}'", electorate_name))
        })?;

        if electorate.is_mutex && groups.len() > 1 {
            return Err(Error::Status(
                Status::UnprocessableEntity,
                format!(
                    "Cannot join more than one group in mutex electorate {}",
                    electorate_name
                ),
            ));
        }

        let invalid_groups: Vec<_> = groups.difference(&electorate.groups).collect();
        if !invalid_groups.is_empty() {
            return Err(Error::not_found(format!(
                "Groups for electorate '{}' with the following names '{:?}'",
                electorate_name, invalid_groups
            )));
        }
    }

    // Find questions restricted to those groups
    let allowed_questions = election
        .questions
        .iter()
        .filter_map(|(question_id, question)| {
            // Index into join object is valid since electorates and groups in it and the question
            // constraints are existence-checked
            question
                .constraints
                .iter()
                .any(|(electorate_name, groups)| {
                    let electorate_joined = joins.get(electorate_name);
                    if let Some(joined) = electorate_joined {
                        !groups.is_disjoint(joined)
                    } else {
                        false
                    }
                })
                .then(|| (*question_id, false))
        })
        .collect::<HashMap<_, _>>();
    let allowed_questions = AllowedQuestions {
        confirmed: allowed_questions,
    };
    let allowed_questions = mongodb::bson::to_bson(&allowed_questions).unwrap(); // Cannot fail.

    // Join the election by adding the voter's unanswered questions
    let allowed_questions_election_id = format!("allowed_questions.{}", election.id);
    voters
        .update_one(
            voter.id.as_doc(),
            doc! {
                "$set": {
                    allowed_questions_election_id: allowed_questions
                }
            },
            None,
        )
        .await?;

    Ok(())
}

#[get("/elections/<election_id>/questions/allowed")]
async fn get_allowed(
    token: AuthToken<Voter>,
    election_id: Id,
    voters: Coll<Voter>,
) -> Result<Json<AllowedQuestions>> {
    let voter = voter_by_id(token.id, &voters).await?;
    // Find what questions they can still vote for.
    let allowed = voter
        .allowed_questions
        .get(&election_id)
        .cloned()
        .unwrap_or_default();

    Ok(Json(allowed))
}

#[post(
    "/elections/<election_id>/votes/cast",
    data = "<ballot_specs>",
    format = "json"
)]
async fn cast_ballots(
    _token: AuthToken<Voter>,
    election_id: Id,
    ballot_specs: Json<Vec<BallotSpec>>,
    elections: Coll<Election>,
    ballots: Coll<NewBallot>,
    counters: Coll<Counter>,
    db_client: &State<Client>,
) -> Result<Json<Vec<Receipt<Unconfirmed>>>> {
    // Get the election.
    let election = active_election_by_id(election_id, &elections).await?;

    // Ensure that the questions and candidates exist.
    for ballot_spec in &*ballot_specs {
        if let Some(question) = election.questions.get(&ballot_spec.question) {
            if !question.candidates.contains(&ballot_spec.candidate) {
                return Err(Error::not_found(format!(
                    "Candidate '{}' for question '{}'",
                    ballot_spec.candidate, ballot_spec.question
                )));
            }
        } else {
            return Err(Error::not_found(format!(
                "Question '{}'",
                ballot_spec.question
            )));
        }
    }

    // Generate cryptographic ballots.
    // The scoped block is needed to force `rng` to be dropped before the next `await`.
    let mut new_ballots = Vec::new();
    {
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

            // Obtain the next ballot ID.
            let ballot_id = Counter::next(&counters, question.id).await?;

            // Create the ballot.
            let ballot = NewBallot::new(
                ballot_id,
                question.id,
                yes_candidate,
                no_candidates,
                &election,
                rand::thread_rng(),
            )
            .ok_or_else(|| {
                Error::Status(
                    Status::InternalServerError,
                    format!("Duplicate candidates for question {}", question.id),
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
    "/elections/<election_id>/votes/audit",
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
                "_id": *audited.internal_id,
                "election_id": election_id,
                "question_id": *audited.question_id,
                "state": Unconfirmed,
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
        .map(|ballot| Receipt::from_ballot(ballot.ballot, &election))
        .collect();

    Ok(Json(receipts))
}

#[post(
    "/elections/<election_id>/votes/confirm",
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
    let mut voter = voter_by_id(token.id, &voters).await?;
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
            let allowed_questions = match voter.allowed_questions.get_mut(&election_id) {
                Some(allowed) => allowed,
                None => {
                    return Err(Error::Status(
                        Status::BadRequest,
                        format!(
                            "Voter {} has not yet joined election {}",
                            voter.id, election_id
                        ),
                    ));
                }
            };
            if let Some(confirmed) = allowed_questions.confirmed.get_mut(&ballot.question_id) {
                if *confirmed {
                    return Err(Error::Status(
                        Status::BadRequest,
                        format!(
                            "Voter {} has already voted on question {}",
                            voter.id, ballot.question_id
                        ),
                    ));
                }

                // All tests passed, the voter can confirm this ballot.
                *confirmed = true; // Not strictly necessary, but best practice to keep our local copy consistent.
                let question_confirmed =
                    format!("allowed_questions.{}.{}", election_id, ballot.question_id);
                let update = doc! {
                    "$set": {
                        &question_confirmed: true
                    }
                };
                let result = voters
                    .update_one_with_session(voter.id.as_doc(), update, None, &mut session)
                    .await?;
                assert_eq!(result.modified_count, 1);
            } else {
                return Err(Error::Status(
                    Status::BadRequest,
                    format!(
                        "Voter {} is not allowed to vote on question {}",
                        voter.id, ballot.question_id
                    ),
                ));
            }

            // Get candidate totals.
            let filter = doc! {
                "election_id": election_id,
                "question_id": *ballot.question_id,
            };
            let mut totals = candidate_totals
                .find(filter, None)
                .await?
                .try_collect::<Vec<_>>()
                .await?;
            // If the totals don't exist yet, we need to create them.
            if totals.len() != ballot.crypto.votes.len() {
                assert_eq!(totals.len(), 0);
                let question = election.questions.get(&ballot.question_id).unwrap();
                for candidate in &question.candidates {
                    totals.push(CandidateTotals {
                        id: Id::new(),
                        totals: NewCandidateTotals::new(
                            election_id,
                            ballot.question_id,
                            candidate.clone(),
                        ),
                    });
                }
            }
            assert_eq!(totals.len(), ballot.crypto.votes.len());
            // Convert to hashmap.
            let mut totals_map = totals
                .iter_mut()
                .map(|t| (t.candidate_name.clone(), &mut t.crypto))
                .collect::<HashMap<_, _>>();

            // Confirm ballot.
            let confirmed = ballot.confirm(&mut totals_map);
            let filter = doc! {
                "_id": *confirmed.internal_id,
                "election_id": election_id,
                "question_id": *confirmed.question_id,
                "state": Unconfirmed,
            };
            let result = confirmed_ballots
                .replace_one_with_session(filter, &confirmed, None, &mut session)
                .await?;
            assert_eq!(result.modified_count, 1);

            // Write updated candidate totals.
            for t in totals {
                let filter = doc! {
                    "election_id": election_id,
                    "question_id": *confirmed.question_id,
                    "candidate_name": t.candidate_name.clone(),
                };
                let options = ReplaceOptions::builder().upsert(true).build();
                let result = candidate_totals
                    .replace_one_with_session(filter, t, options, &mut session)
                    .await?;
                assert!(result.modified_count == 1 || result.upserted_id.is_some());
            }

            new_ballots.push(confirmed);
        }

        session.commit_transaction().await?;
    }

    // Return receipts.
    let receipts = new_ballots
        .into_iter()
        .map(|ballot| Receipt::from_ballot(ballot.ballot, &election))
        .collect();

    Ok(Json(receipts))
}

async fn voter_by_id(voter_id: Id, voters: &Coll<Voter>) -> Result<Voter> {
    voters
        .find_one(voter_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Voter with ID {}", voter_id)))
}

/// Return an active Election from the database via ID lookup.
/// An active election is finalised and within its start and end times.
async fn active_election_by_id(election_id: Id, elections: &Coll<Election>) -> Result<Election> {
    let now = Utc::now();

    let is_active = doc! {
        "_id": election_id,
        "state": ElectionState::Published,
        "start_time": { "$lte": now },
        "end_time": { "$gt": now },
    };

    elections
        .find_one(is_active, None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Active election with ID '{}'", election_id)))
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
            "ballot_id": recall.ballot_id as i64, // BSON technically doesn't support u64, but i64 conversion is lossless.
            "election_id": election.id,
            "question_id": recall.question_id,
            "state": Unconfirmed,
        };
        let ballot = unconfirmed_ballots
            .find_one(filter, None)
            .await?
            .filter(|ballot| {
                // Verify ownership of the ballot. If this fails, we return
                // an error indistinguishable from the ballot ID not existing,
                // so an attacker cannot learn anything about valid ballot IDs.
                let true_signature =
                    Receipt::from_ballot(ballot.ballot.clone(), election).signature;
                true_signature == recall.signature
            })
            .ok_or_else(|| Error::not_found(format!("Ballot with ID '{}'", recall.ballot_id)))?;
        ballots.push(ballot);
    }
    Ok(ballots)
}

#[cfg(test)]
mod tests {
    use backend_test::backend_test;
    use chrono::{Duration, Utc};
    use dre_ip::{DreipPublicKey, DreipScalar, Serializable};
    use mongodb::{bson::to_bson, Database};
    use rand::Rng;
    use rocket::{
        futures::{StreamExt, TryStreamExt},
        http::ContentType,
        local::asynchronous::Client,
        serde::json::serde_json,
    };

    use crate::model::api::election::ElectionDescription;
    use crate::model::{
        api::{
            election::{ElectionResults, QuestionSpec},
            receipt::Signature,
            sms::Sms,
        },
        common::ballot::{Audited, Confirmed, Unconfirmed},
        db::election::{Election, NewElection},
    };

    use super::*;

    /// Insert test data, returning the election ID and the ID of the allowed question,
    /// which will differ between runs.
    async fn insert_test_data(client: &Client, db: &Database) -> (Id, Id) {
        // Create some elections, only one of which is active.
        let election1 = Election {
            id: Id::new(),
            election: NewElection::published_example(),
        };
        let election2 = Election {
            id: Id::new(),
            election: NewElection::draft_example(),
        };
        let elections = Coll::<Election>::from_db(db);
        elections
            .insert_many(vec![&election1, &election2], None)
            .await
            .unwrap();

        // Create the associated counters.
        let mut counters = Vec::new();
        for election in [&election1, &election2] {
            for question_id in election.questions.keys() {
                let counter = Counter::new(*question_id, 1);
                counters.push(counter);
            }
        }
        Coll::<Counter>::from_db(db)
            .insert_many(counters, None)
            .await
            .unwrap();

        // Allow the voter to vote on one of the two questions.
        let voters = Coll::<Voter>::from_db(db);
        let mut voter = voters
            .find_one(
                doc! {
                    "sms_hmac": to_bson(&Sms::example_hmac(client)).unwrap(),
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
        voter.allowed_questions.insert(
            election1.id,
            AllowedQuestions {
                confirmed: HashMap::from_iter(vec![(allowed_question, false)]),
            },
        );
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
    #[allow(dead_code)]
    async fn dump_db_state(db: &Database) {
        println!("\nVoters:");
        let mut voters = Coll::<Voter>::from_db(db).find(None, None).await.unwrap();
        while let Some(Ok(voter)) = voters.next().await {
            println!("{:?}", voter);
        }

        println!("\nElections:");
        let mut elections = Coll::<Election>::from_db(db)
            .find(None, None)
            .await
            .unwrap();
        while let Some(Ok(election)) = elections.next().await {
            println!("{:?}", election);
        }

        println!("\nUnconfirmed Ballots:");
        let mut ballots = Coll::<Ballot<Unconfirmed>>::from_db(db)
            .find(doc! {"state": "Unconfirmed"}, None)
            .await
            .unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }
        println!("\nAudited Ballots:");
        let mut ballots = Coll::<Ballot<Audited>>::from_db(db)
            .find(doc! {"state": "Audited"}, None)
            .await
            .unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }
        println!("\nConfirmed Ballots:");
        let mut ballots = Coll::<Ballot<Confirmed>>::from_db(db)
            .find(doc! {"state": "Confirmed"}, None)
            .await
            .unwrap();
        while let Some(Ok(ballot)) = ballots.next().await {
            println!("{:?}", ballot);
        }

        println!("\nCandidate Totals:");
        let mut totals = Coll::<CandidateTotals>::from_db(db)
            .find(None, None)
            .await
            .unwrap();
        while let Some(Ok(total)) = totals.next().await {
            println!("{:?}", total);
        }
    }

    #[backend_test(voter)]
    async fn has_joined(client: Client, db: Database) {
        let election = NewElection::published_example();
        let election_id: Id = Coll::<NewElection>::from_db(&db)
            .insert_one(&election, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .into();

        // We haven't yet joined the election, so the endpoint should agree.
        let response = client.get(uri!(has_joined(election_id))).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());
        let joined: bool = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        assert!(!joined);

        // Join the election (no groups).
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body("{}")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_none());

        // The endpoint should now say we have joined.
        let response = client.get(uri!(has_joined(election_id))).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());
        let joined: bool = serde_json::from_str(&response.into_string().await.unwrap()).unwrap();
        assert!(joined);
    }

    #[backend_test(voter)]
    async fn join_all_groups(client: Client, db: Database) {
        let election = NewElection::published_example();
        let election_id: Id = Coll::<NewElection>::from_db(&db)
            .insert_one(&election, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .into();

        // Check no questions are allowed.
        let sms_hmac = Sms::example_hmac(&client);
        let voter = Coll::<Voter>::from_db(&db)
            .find_one(doc! {"sms_hmac": to_bson(&sms_hmac).unwrap()}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(voter.allowed_questions, HashMap::new());

        // Join as many groups as we can.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![
            (
                "Societies".to_string(),
                HashSet::from_iter(vec!["Quidditch", "Moongolf"].into_iter().map(String::from)),
            ),
            (
                "Courses".to_string(),
                HashSet::from_iter(vec!["CompSci".to_string()]),
            ),
        ]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_none());

        // Check all questions are allowed.
        let voter = Coll::<Voter>::from_db(&db)
            .find_one(doc! {"sms_hmac": to_bson(&sms_hmac).unwrap()}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            voter.allowed_questions[&election_id]
                .keys()
                .collect::<HashSet<_>>(),
            election.questions.keys().collect()
        );
    }

    #[backend_test(voter)]
    async fn join_one_group(client: Client, db: Database) {
        let election = NewElection::published_example();
        let election_id: Id = Coll::<NewElection>::from_db(&db)
            .insert_one(&election, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .into();

        // Check no questions are allowed.
        let sms_hmac = Sms::example_hmac(&client);
        let voter = Coll::<Voter>::from_db(&db)
            .find_one(doc! {"sms_hmac": to_bson(&sms_hmac).unwrap()}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(voter.allowed_questions, HashMap::new());

        // Join one group only.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Societies".to_string(),
            HashSet::from_iter(vec!["Quidditch".to_string()]),
        )]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_none());

        // Check the correct question is allowed.
        let voter = Coll::<Voter>::from_db(&db)
            .find_one(doc! {"sms_hmac": to_bson(&sms_hmac).unwrap()}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            voter.allowed_questions[&election_id]
                .keys()
                .collect::<HashSet<_>>(),
            election
                .questions
                .iter()
                .filter_map(
                    |(id, q)| if q.description == QuestionSpec::example1().description {
                        Some(id)
                    } else {
                        None
                    }
                )
                .collect()
        );
    }

    #[backend_test(voter)]
    async fn bad_joins(client: Client, db: Database) {
        let election = NewElection::published_example();
        let election_id: Id = Coll::<NewElection>::from_db(&db)
            .insert_one(election, None)
            .await
            .unwrap()
            .inserted_id
            .as_object_id()
            .unwrap()
            .into();

        // Try to join a non-existent election.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Societies".to_string(),
            HashSet::from_iter(vec!["Quidditch".to_string()]),
        )]);
        let response = client
            .post(uri!(join_election(Id::new())))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to join a non-existent electorate.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Foo".to_string(),
            HashSet::from_iter(vec!["Quidditch".to_string()]),
        )]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to join a non-existent group.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Societies".to_string(),
            HashSet::from_iter(vec!["Foo".to_string()]),
        )]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to join two mutually-exclusive groups.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Courses".to_string(),
            HashSet::from_iter(vec!["CompSci", "Maths"].into_iter().map(String::from)),
        )]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::UnprocessableEntity);

        // Try to join an election twice.
        let joins: HashMap<String, HashSet<String>> = HashMap::from_iter(vec![(
            "Societies".to_string(),
            HashSet::from_iter(vec!["Quidditch".to_string()]),
        )]);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        let response = client
            .post(uri!(join_election(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&joins).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Forbidden);
    }

    #[backend_test(voter)]
    async fn get_allowed(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

        // Get the allowed questions.
        let response = client.get(uri!(get_allowed(election_id))).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        // Ensure they are correct.
        let raw_response = response.into_string().await.unwrap();
        let allowed: AllowedQuestions = serde_json::from_str(&raw_response).unwrap();
        let expected = HashMap::from_iter(vec![(question_id, false)]);
        assert_eq!(allowed.confirmed, expected);
    }

    #[backend_test(voter)]
    async fn cast_ballots(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

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
        let receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Ensure the receipt passes validation.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc! {"_id": election_id}, None)
            .await
            .unwrap()
            .unwrap();
        // Validate PWFs.
        assert!(receipt
            .crypto
            .verify(
                election.crypto.g1,
                election.crypto.g2,
                receipt.ballot_id.to_le_bytes()
            )
            .is_ok());
        // Validate signature.
        let mut msg = receipt.crypto.to_bytes();
        msg.extend(receipt.ballot_id.to_le_bytes());
        msg.extend(receipt.election_id.to_bytes());
        msg.extend(receipt.question_id.to_bytes());
        msg.extend(receipt.state.as_ref());
        msg.extend(Into::<Vec<u8>>::into(&receipt.state_data));
        assert!(election.crypto.public_key.verify(&msg, &receipt.signature));

        // Ensure the ballot in the database is correct.
        let ballot = Coll::<Ballot<Unconfirmed>>::from_db(&db)
            .find_one(
                doc! {
                    "ballot_id": receipt.ballot_id as i64,
                    "election_id": *receipt.election_id,
                    "question_id": *receipt.question_id,
                },
                None,
            )
            .await
            .unwrap()
            .unwrap();
        // Check metadata.
        assert_eq!(ballot.election_id, election_id);
        assert_eq!(ballot.question_id, question_id);
        assert!(
            ballot.creation_time < Utc::now()
                && ballot.creation_time + Duration::minutes(1) > Utc::now()
        );
        let mut yes_votes = 0;
        for (candidate, vote) in ballot.crypto.votes.iter() {
            // Check the vote value is correct.
            if candidate == &candidate_id {
                assert_eq!(vote.secrets.v, DreipScalar::one());
                yes_votes += 1;
            } else {
                assert_eq!(vote.secrets.v, DreipScalar::zero());
            }
            // Check the receipt matches the internal vote.
            let receipt_vote = receipt.crypto.votes.get(candidate).unwrap();
            assert_eq!(vote.R, receipt_vote.R);
            assert_eq!(vote.Z, receipt_vote.Z);
            assert_eq!(vote.pwf, receipt_vote.pwf);
            // Check the public values were correctly calculated from the secrets.
            assert_eq!(vote.R, election.crypto.g2 * vote.secrets.r);
            assert_eq!(
                vote.Z,
                election.crypto.g1 * (vote.secrets.r + vote.secrets.v)
            );
        }
        // Check there was exactly one yes vote (already guaranteed by the PWF, but sanity checks are good).
        assert_eq!(yes_votes, 1);
    }

    #[backend_test(voter)]
    async fn audit(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

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
        let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Audit the ballot.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.ballot_id,
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
        let second_receipt: Receipt<Audited> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Ensure the receipts match.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc! {"_id": election_id}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first_receipt.ballot_id, second_receipt.ballot_id);
        assert_eq!(first_receipt.crypto.pwf, second_receipt.crypto.pwf);
        for (candidate, vote1) in first_receipt.crypto.votes.iter() {
            let vote2 = second_receipt.crypto.votes.get(candidate).unwrap();
            assert_eq!(vote1.pwf, vote2.pwf);
            assert_eq!(vote1.R, vote2.R);
            assert_eq!(vote1.Z, vote2.Z);
            assert_eq!(vote1.R, election.crypto.g2 * vote2.secrets.r);
            assert_eq!(
                vote1.Z,
                election.crypto.g1 * (vote2.secrets.r + vote2.secrets.v)
            );
        }

        // Validate PWFs.
        assert!(second_receipt
            .crypto
            .verify(
                election.crypto.g1,
                election.crypto.g2,
                second_receipt.ballot_id.to_le_bytes()
            )
            .is_ok());
        // Validate signature.
        let mut msg = second_receipt.crypto.to_bytes();
        msg.extend(second_receipt.ballot_id.to_le_bytes());
        msg.extend(second_receipt.election_id.to_bytes());
        msg.extend(second_receipt.question_id.to_bytes());
        msg.extend(second_receipt.state.as_ref());
        msg.extend(Into::<Vec<u8>>::into(&second_receipt.state_data));
        assert!(election
            .crypto
            .public_key
            .verify(&msg, &second_receipt.signature));

        // Check the candidate totals weren't affected.
        let totals: Vec<CandidateTotals> = Coll::<CandidateTotals>::from_db(&db)
            .find(None, None)
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        for total in totals.iter() {
            assert_eq!(total.crypto.r_sum, DreipScalar::zero());
            assert_eq!(total.crypto.tally, DreipScalar::zero());
        }
    }

    #[backend_test(voter)]
    async fn confirm(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

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
        let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Confirm the ballot.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.ballot_id,
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
        let second_receipt: Receipt<Confirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Ensure the receipts match.
        let election = Coll::<Election>::from_db(&db)
            .find_one(doc! {"_id": election_id}, None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first_receipt.ballot_id, second_receipt.ballot_id);
        assert_eq!(first_receipt.crypto, second_receipt.crypto);

        // Validate PWFs.
        assert!(second_receipt
            .crypto
            .verify(
                election.crypto.g1,
                election.crypto.g2,
                second_receipt.ballot_id.to_le_bytes()
            )
            .is_ok());
        // Validate signature.
        let mut msg = second_receipt.crypto.to_bytes();
        msg.extend(second_receipt.ballot_id.to_le_bytes());
        msg.extend(second_receipt.election_id.to_bytes());
        msg.extend(second_receipt.question_id.to_bytes());
        msg.extend(second_receipt.state.as_ref());
        msg.extend(Into::<Vec<u8>>::into(&second_receipt.state_data));
        assert!(election
            .crypto
            .public_key
            .verify(&msg, &second_receipt.signature));

        // Check the candidate totals are correct.
        let candidate_totals: Vec<CandidateTotals> = Coll::<CandidateTotals>::from_db(&db)
            .find(None, None)
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();
        for total in candidate_totals.iter() {
            if total.question_id == question_id && total.candidate_name == candidate_id {
                assert_eq!(total.crypto.tally, DreipScalar::one());
            } else {
                assert_eq!(total.crypto.tally, DreipScalar::zero());
            }
        }
        let mut ballots = HashMap::new();
        ballots.insert(second_receipt.ballot_id, second_receipt);
        let mut totals = HashMap::new();
        for total in candidate_totals {
            totals.insert(total.candidate_name.clone(), total.into());
        }
        let results = ElectionResults {
            election: ElectionDescription::from(election).crypto,
            audited: HashMap::new(),
            confirmed: ballots,
            totals,
        };

        assert!(results.verify().is_ok());

        // Ensure the question is marked as answered.
        let response = client.get(uri!(get_allowed(election_id))).dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let raw_response = response.into_string().await.unwrap();
        let allowed: AllowedQuestions = serde_json::from_str(&raw_response).unwrap();
        assert!(allowed.confirmed[&question_id]);
    }

    #[backend_test(voter)]
    async fn bad_casts(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

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
            .find_one(doc! {"state": ElectionState::Draft}, None)
            .await
            .unwrap()
            .unwrap();
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
            .find(None, None)
            .await
            .unwrap();
        assert_eq!(ballots.try_collect::<Vec<_>>().await.unwrap().len(), 0);
    }

    #[backend_test(voter)]
    async fn bad_votes(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

        // Vote on an inactive election.
        let inactive_election = Coll::<Election>::from_db(&db)
            .find_one(doc! {"_id": {"$ne": election_id}}, None)
            .await
            .unwrap()
            .unwrap();
        let question = *inactive_election.questions.keys().next().unwrap();
        let ballot_specs = vec![BallotSpec {
            question,
            candidate: inactive_election
                .questions
                .get(&question)
                .unwrap()
                .candidates[0]
                .clone(),
        }];
        let response = client
            .post(uri!(cast_ballots(inactive_election.id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Vote on a non-existent election.
        let response = client
            .post(uri!(cast_ballots(Id::new())))
            .header(ContentType::JSON)
            .body("[]")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Vote on a non-existent question.
        let ballot_specs = vec![BallotSpec {
            question: Id::new(),
            candidate: "Alice".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Vote on the question we are not allowed to.
        let not_allowed_question = *Coll::<Election>::from_db(&db)
            .find_one(election_id.as_doc(), None)
            .await
            .unwrap()
            .unwrap()
            .questions
            .iter()
            .find_map(|(id, q)| {
                if q.description == QuestionSpec::example2().description {
                    Some(id)
                } else {
                    None
                }
            })
            .unwrap();
        let ballot_specs = vec![BallotSpec {
            question: not_allowed_question,
            candidate: "Alice".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Vote for a non-existent candidate.
        let ballot_specs = vec![BallotSpec {
            question: question_id,
            candidate: "Alice".to_string(),
        }];
        let response = client
            .post(uri!(cast_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_specs).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

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
        let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        // Try to confirm the wrong ballot ID.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: rand::thread_rng().gen(),
            question_id,
            signature: first_receipt.signature,
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to confirm the wrong question ID.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.ballot_id,
            question_id: Id::new(),
            signature: first_receipt.signature,
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to confirm the wrong signature.
        let mut signature = first_receipt.signature.to_bytes();
        signature[0] = signature[0].wrapping_add(1);
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.ballot_id,
            question_id,
            signature: Signature::from_bytes(&signature).unwrap(),
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Correctly confirm.
        let ballot_recalls = vec![BallotRecall {
            ballot_id: first_receipt.ballot_id,
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

        // Try to confirm again.
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Try to audit after confirming.
        let response = client
            .post(uri!(audit_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[backend_test(voter)]
    async fn cant_vote_twice(client: Client, db: Database) {
        let (election_id, question_id) = insert_test_data(&client, &db).await;

        // Correctly cast a vote.
        async fn cast(client: &Client, election_id: Id, question_id: Id) -> Receipt<Unconfirmed> {
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
            let raw_response = response.into_string().await.unwrap();
            let first_receipt: Receipt<Unconfirmed> = serde_json::from_str::<Vec<_>>(&raw_response)
                .unwrap()
                .into_iter()
                .next()
                .unwrap();
            first_receipt
        }

        // Confirm a vote.
        let receipt = cast(&client, election_id, question_id).await;
        let ballot_recalls = vec![BallotRecall {
            ballot_id: receipt.ballot_id,
            question_id,
            signature: receipt.signature,
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        // Cast an identical vote, and audit it.
        let receipt = cast(&client, election_id, question_id).await;
        let ballot_recalls = vec![BallotRecall {
            ballot_id: receipt.ballot_id,
            question_id,
            signature: receipt.signature,
        }];
        let response = client
            .post(uri!(audit_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);

        // Ensure we can't confirm a second vote though.
        let receipt = cast(&client, election_id, question_id).await;
        let ballot_recalls = vec![BallotRecall {
            ballot_id: receipt.ballot_id,
            question_id,
            signature: receipt.signature,
        }];
        let response = client
            .post(uri!(confirm_ballots(election_id)))
            .header(ContentType::JSON)
            .body(serde_json::to_string(&ballot_recalls).unwrap())
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::BadRequest);

        // Ensure there are the expected votes present.
        let ballots = Coll::<Ballot<Unconfirmed>>::from_db(&db);
        let total_num = ballots.count_documents(None, None).await.unwrap();
        assert_eq!(total_num, 3);
        let unconfirmed_num = ballots
            .count_documents(doc! {"state": Unconfirmed}, None)
            .await
            .unwrap();
        assert_eq!(unconfirmed_num, 1);
        let audited_num = ballots
            .count_documents(doc! {"state": Audited}, None)
            .await
            .unwrap();
        assert_eq!(audited_num, 1);
        let confirmed_num = ballots
            .count_documents(doc! {"state": Confirmed}, None)
            .await
            .unwrap();
        assert_eq!(confirmed_num, 1);
    }
}
