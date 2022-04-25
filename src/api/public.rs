use std::collections::HashMap;

use chrono::Utc;
use mongodb::{
    bson::doc,
    options::{FindOptions, SessionOptions},
    Client,
};
use rocket::{
    futures::{StreamExt, TryStreamExt},
    serde::json::Json,
    Route, State,
};

use crate::error::{Error, Result};
use crate::model::{
    api::{
        auth::AuthToken,
        candidate_totals::CandidateTotalsDesc,
        election::{ElectionDescription, ElectionResults, ElectionSummary},
        pagination::{Paginated, PaginationRequest},
        receipt::{PublicReceipt, Receipt},
    },
    common::{
        ballot::{Audited, BallotId, Confirmed},
        election::{CandidateId, ElectionId, ElectionState, QuestionId},
    },
    db::{admin::Admin, ballot::AnyBallot, candidate_totals::CandidateTotals, election::Election},
    mongodb::{u32_id_filter, Coll},
};

pub fn routes() -> Vec<Route> {
    routes![
        elections_admin,
        elections_non_admin,
        election_admin,
        election_non_admin,
        election_question_ballots,
        election_question_ballot,
        candidate_totals,
        question_dump,
    ]
}

#[get("/elections?<archived>", rank = 1)]
async fn elections_admin(
    _token: AuthToken<Admin>,
    archived: Option<bool>,
    elections: Coll<Election>,
) -> Result<Json<Vec<ElectionSummary>>> {
    let archived = archived.unwrap_or(false);
    metadata_for_elections(elections, true, archived).await
}

#[get("/elections?<archived>", rank = 2)]
async fn elections_non_admin(
    archived: Option<bool>,
    elections: Coll<Election>,
) -> Result<Json<Vec<ElectionSummary>>> {
    let archived = archived.unwrap_or(false);
    metadata_for_elections(elections, false, archived).await
}

#[get("/elections/<election_id>", rank = 1)]
async fn election_admin(
    _token: AuthToken<Admin>,
    election_id: ElectionId,
    elections: Coll<Election>,
) -> Result<Json<ElectionDescription>> {
    let election = elections
        .find_one(u32_id_filter(election_id), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;
    Ok(Json(election.into()))
}

#[get("/elections/<election_id>", rank = 2)]
async fn election_non_admin(
    election_id: ElectionId,
    elections: Coll<Election>,
) -> Result<Json<ElectionDescription>> {
    let filter = doc! {
        "_id": election_id,
        "$or": [{"state": ElectionState::Published}, {"state": ElectionState::Archived}],
    };

    let election = elections
        .find_one(filter, None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Non-admin election with ID '{}'", election_id)))?;

    Ok(Json(election.into()))
}

#[get("/elections/<election_id>/<question_id>/ballots?<filter_pattern>&<pagination..>")]
async fn election_question_ballots(
    election_id: ElectionId,
    question_id: QuestionId,
    filter_pattern: Option<String>,
    pagination: PaginationRequest,
    elections: Coll<Election>,
    ballots: Coll<AnyBallot>,
) -> Result<Json<Paginated<PublicReceipt>>> {
    // No need to filter our drafts if non-admin, since draft elections cannot have ballots.
    let election = elections
        .find_one(u32_id_filter(election_id), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let mut filter = doc! {
        "election_id": election_id,
        "question_id": question_id,
    };
    if let Some(pattern) = filter_pattern {
        filter.insert(
            "$expr",
            doc! {
                "$regexMatch": {
                    "input": {"$toString": "$ballot_id"},
                    "regex": pattern,
                }
            },
        );
    }

    let pagination_options = FindOptions::builder()
        .skip(u64::from(pagination.skip()))
        .limit(i64::from(pagination.page_size()))
        .build();

    let ballots_page = ballots
        .find(filter.clone(), pagination_options)
        .await?
        .map(|ballot| ballot.map(|ballot| PublicReceipt::from_ballot(ballot, &election)))
        .try_collect::<Vec<_>>()
        .await?;

    let total_ballots = ballots.count_documents(filter, None).await?;

    let paginated = pagination.to_paginated(total_ballots, ballots_page);
    Ok(Json(paginated))
}

#[get("/elections/<election_id>/<question_id>/ballots/<ballot_id>")]
async fn election_question_ballot(
    election_id: ElectionId,
    question_id: QuestionId,
    ballot_id: BallotId,
    elections: Coll<Election>,
    ballots: Coll<AnyBallot>,
) -> Result<Json<PublicReceipt>> {
    // No need to filter our drafts if non-admin, since draft elections cannot have ballots.
    let election = elections
        .find_one(u32_id_filter(election_id), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let election_question_ballot = doc! {
        "ballot_id": ballot_id,
        "election_id": election_id,
        "question_id": question_id,
    };

    let ballot = ballots
        .find_one(election_question_ballot, None)
        .await?
        .map(|ballot| PublicReceipt::from_ballot(ballot, &election))
        .ok_or_else(|| {
            Error::not_found(format!(
                "Ballot with ID '{}' for election '{}', question '{}'",
                ballot_id, election_id, question_id
            ))
        })?;

    Ok(Json(ballot))
}

#[get("/elections/<election_id>/<question_id>/totals")]
async fn candidate_totals(
    election_id: ElectionId,
    question_id: QuestionId,
    elections: Coll<Election>,
    totals: Coll<CandidateTotals>,
) -> Result<Json<HashMap<CandidateId, CandidateTotalsDesc>>> {
    let election = elections
        .find_one(u32_id_filter(election_id), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    if election.metadata.state != ElectionState::Archived
        && Utc::now() <= election.metadata.end_time
    {
        return Err(Error::not_found(format!(
            "Election with ID '{}'",
            election_id
        )));
    }

    let question_totals_filter = doc! {
        "election_id": election_id,
        "question_id": question_id,
    };
    let question_totals = totals
        .find(question_totals_filter, None)
        .await?
        .map_ok(|tot| (tot.candidate_name.clone(), tot.into()))
        .try_collect::<HashMap<_, _>>()
        .await?;

    Ok(Json(question_totals))
}

#[get("/elections/<election_id>/<question_id>/dump")]
async fn question_dump(
    election_id: ElectionId,
    question_id: QuestionId,
    elections: Coll<Election>,
    totals: Coll<CandidateTotals>,
    ballots: Coll<AnyBallot>,
    db_client: &State<Client>,
) -> Result<Json<ElectionResults>> {
    // Ensure we read a consistent snapshot of the election data.
    let election;
    let mut candidate_totals = None;
    let mut audited_receipts = HashMap::new();
    let mut confirmed_receipts = HashMap::new();
    {
        let session_options = SessionOptions::builder().snapshot(true).build();
        let mut session = db_client.start_session(Some(session_options)).await?;

        let election_filter = doc! {
            "_id": election_id,
            "$or": [{"state": ElectionState::Published}, {"state": ElectionState::Archived}],
        };
        election = elections
            .find_one_with_session(election_filter, None, &mut session)
            .await?
            .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

        // Only retrieve totals if the election has finished.
        if election.metadata.state == ElectionState::Archived
            || Utc::now() > election.metadata.end_time
        {
            let totals_filter = doc! {
                "election_id": election_id,
                "question_id": question_id,
            };
            let mut totals_cursor = totals
                .find_with_session(totals_filter, None, &mut session)
                .await?;
            candidate_totals = Some({
                let mut candidate_totals = HashMap::new();
                while let Some(total) = totals_cursor.next(&mut session).await {
                    let total = total?;
                    candidate_totals.insert(total.candidate_name.clone(), total.into());
                }
                candidate_totals
            });
        }

        let ballots_filter = doc! {
            "election_id": election_id,
            "question_id": question_id,
            "$or": [{"state": Audited}, {"state": Confirmed}],
        };
        let mut election_ballots = ballots
            .find_with_session(ballots_filter, None, &mut session)
            .await?;
        while let Some(ballot) = election_ballots.next(&mut session).await {
            match ballot? {
                AnyBallot::Unconfirmed(_) => {} // Ignore unconfirmed ballots.
                AnyBallot::Audited(b) => {
                    audited_receipts.insert(b.ballot_id, Receipt::from_ballot(b.ballot, &election));
                }
                AnyBallot::Confirmed(b) => {
                    confirmed_receipts
                        .insert(b.ballot_id, Receipt::from_ballot(b.ballot, &election));
                }
            }
        }
    }

    let dump = ElectionResults {
        election: ElectionDescription::from(election).crypto,
        audited: audited_receipts,
        confirmed: confirmed_receipts,
        totals: candidate_totals,
    };

    Ok(Json(dump))
}

/// Retrieve the metadata for elections.
/// If `admin` is false, admin-only elections will be hidden.
/// If `archived` is true, archived elections will be returned instead of non-archived ones.
async fn metadata_for_elections(
    elections: Coll<Election>,
    admin: bool,
    archived: bool,
) -> Result<Json<Vec<ElectionSummary>>> {
    let filter = if archived {
        doc! {
            "state": ElectionState::Archived,
        }
    } else if admin {
        doc! {
            "$or": [{"state": ElectionState::Draft}, {"state": ElectionState::Published}],
        }
    } else {
        doc! {
            "state": ElectionState::Published,
        }
    };

    let elections = elections
        .find(filter, None)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let metadata = elections.into_iter().map(Into::into).collect();

    Ok(Json(metadata))
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json};
    use std::collections::HashMap;

    use crate::model::{
        api::election::{ElectionSpec, QuestionSpec},
        common::ballot::Unconfirmed,
        db::{
            ballot::{Ballot, BallotCore},
            candidate_totals::NewCandidateTotals,
        },
    };

    use super::*;

    #[backend_test(admin)]
    async fn get_all_elections_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let response = client
            .get(uri!(elections_admin(Some(false))))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let fetched_elections = serde_json::from_str::<Vec<ElectionSummary>>(&raw_response)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();

        let expected = vec![
            Election::published_example().metadata,
            Election::draft_example().metadata,
        ];
        for (actual, expected) in std::iter::zip(fetched_elections, expected) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.state, expected.state);
            assert_eq!(actual.start_time, expected.start_time);
            assert_eq!(actual.end_time, expected.end_time);
        }
    }

    #[backend_test]
    async fn only_get_published_elections_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let response = client
            .get(uri!(elections_non_admin(Some(false))))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        let fetched_elections = serde_json::from_str::<Vec<ElectionSummary>>(&raw_response)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();

        let expected = vec![Election::published_example().metadata];
        for (actual, expected) in std::iter::zip(fetched_elections, expected) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.state, expected.state);
            assert_eq!(actual.start_time, expected.start_time);
            assert_eq!(actual.end_time, expected.end_time);
        }
    }

    #[backend_test(admin)]
    async fn get_published_election_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let response = client
            .get(uri!(election_admin(election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        // Ensure we didn't expose any secrets.
        let error = serde_json::from_str::<Election>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionDescription>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = Election::published_example();

        assert_eq!(expected.metadata.name, fetched_election.name);
        assert_eq!(expected.metadata.state, fetched_election.state);
        assert_eq!(expected.metadata.start_time, fetched_election.start_time);
        assert_eq!(expected.metadata.end_time, fetched_election.end_time);
        assert_eq!(expected.electorates, fetched_election.electorates);
        for question in expected.questions.values() {
            let matching = fetched_election.questions.values().find(|q| {
                // Compare everything except the IDs.
                q.description == question.description
                    && q.candidates == question.candidates
                    && q.constraints == question.constraints
            });
            assert!(matching.is_some());
        }
    }

    #[backend_test(admin)]
    async fn get_draft_election_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::future_example()).await;

        let response = client
            .get(uri!(election_admin(election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        // Ensure we didn't expose any secrets.
        let error = serde_json::from_str::<Election>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionDescription>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = Election::draft_example();

        assert_eq!(expected.metadata.name, fetched_election.name);
        assert_eq!(expected.metadata.state, fetched_election.state);
        assert_eq!(expected.metadata.start_time, fetched_election.start_time);
        assert_eq!(expected.metadata.end_time, fetched_election.end_time);
        assert_eq!(expected.electorates, fetched_election.electorates);
        for question in expected.questions.values() {
            let matching = fetched_election.questions.values().find(|q| {
                // Compare everything except the IDs.
                q.description == question.description
                    && q.candidates == question.candidates
                    && q.constraints == question.constraints
            });
            assert!(matching.is_some());
        }
    }

    #[backend_test]
    async fn get_published_election_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let response = client
            .get(uri!(election_non_admin(election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        // Ensure we didn't expose any secrets.
        let error = serde_json::from_str::<Election>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionDescription>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = Election::published_example();

        assert_eq!(expected.metadata.name, fetched_election.name);
        assert_eq!(expected.metadata.state, fetched_election.state);
        assert_eq!(expected.metadata.start_time, fetched_election.start_time);
        assert_eq!(expected.metadata.end_time, fetched_election.end_time);
        assert_eq!(expected.electorates, fetched_election.electorates);
        for question in expected.questions.values() {
            let matching = fetched_election.questions.values().find(|q| {
                // Compare everything except the IDs.
                q.description == question.description
                    && q.candidates == question.candidates
                    && q.constraints == question.constraints
            });
            assert!(matching.is_some());
        }
    }

    #[backend_test]
    async fn fail_to_get_draft_election_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::future_example()).await;

        let response = client
            .get(uri!(election_non_admin(election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::NotFound, response.status());
    }

    #[backend_test]
    async fn get_archived(client: Client, db: Database) {
        insert_elections(&db).await;

        // Try getting all archived.
        let response = client
            .get(uri!(elections_non_admin(Some(true))))
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let fetched_elections = serde_json::from_str::<Vec<ElectionSummary>>(&raw_response)
            .unwrap()
            .into_iter()
            .collect::<Vec<_>>();

        let expected = vec![Election::archived_example().metadata];
        for (actual, expected) in std::iter::zip(fetched_elections, expected) {
            assert_eq!(actual.name, expected.name);
            assert_eq!(actual.state, expected.state);
            assert_eq!(actual.start_time, expected.start_time);
            assert_eq!(actual.end_time, expected.end_time);
        }

        // Try getting a specific archived election.
        let election_id = serde_json::from_str::<Vec<ElectionSummary>>(&raw_response)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .id;
        let response = client
            .get(uri!(election_non_admin(election_id)))
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        // Ensure we didn't expose any secrets.
        let error = serde_json::from_str::<Election>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionDescription>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = Election::archived_example();

        assert_eq!(expected.metadata.name, fetched_election.name);
        assert_eq!(expected.metadata.state, fetched_election.state);
        assert_eq!(expected.metadata.start_time, fetched_election.start_time);
        assert_eq!(expected.metadata.end_time, fetched_election.end_time);
        assert_eq!(expected.electorates, fetched_election.electorates);
        for question in expected.questions.values() {
            let matching = fetched_election.questions.values().find(|q| {
                // Compare everything except the IDs.
                q.description == question.description
                    && q.candidates == question.candidates
                    && q.constraints == question.constraints
            });
            assert!(matching.is_some());
        }
    }

    #[backend_test]
    async fn get_election_question_ballots(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;
        let question_id = *election
            .questions
            .iter()
            .find_map(|(id, q)| {
                if q.description == QuestionSpec::example1().description {
                    Some(id)
                } else {
                    None
                }
            })
            .unwrap();

        // Get the first page of two.
        let pagination = PaginationRequest {
            page_num: 1,
            page_size: 2,
        };
        let response = client
            .get(uri!(election_question_ballots(
                election.id,
                question_id,
                Option::<String>::None,
                pagination
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipts: Paginated<PublicReceipt> = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(receipts.pagination.page_num, 1);
        assert_eq!(receipts.pagination.page_size, 2);
        assert_eq!(receipts.pagination.total, 9);
        assert_eq!(receipts.items.len(), 2);

        // Get all ballots.
        let pagination = PaginationRequest {
            page_num: 1,
            page_size: 50,
        };
        let response = client
            .get(uri!(election_question_ballots(
                election.id,
                question_id,
                Option::<String>::None,
                pagination
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipts: Paginated<PublicReceipt> = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(receipts.pagination.page_num, 1);
        assert_eq!(receipts.pagination.page_size, 50);
        assert_eq!(receipts.pagination.total, 9);
        assert_eq!(receipts.items.len(), 9);
        assert_eq!(
            receipts
                .items
                .iter()
                .filter(|receipt| matches!(receipt, PublicReceipt::Confirmed(_)))
                .count(),
            5
        );
    }

    #[backend_test]
    async fn get_election_question_ballots_filter(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;
        let question_id = *election
            .questions
            .iter()
            .find_map(|(id, q)| {
                if q.description == QuestionSpec::example1().description {
                    Some(id)
                } else {
                    None
                }
            })
            .unwrap();

        // Filter to ballot IDs containing "3".
        // We expect to see only one: a confirmed ballot with ID 3.
        let pagination = PaginationRequest {
            page_num: 1,
            page_size: 50,
        };
        let response = client
            .get(uri!(election_question_ballots(
                election.id,
                question_id,
                Some("3".to_string()),
                pagination
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipts: Paginated<PublicReceipt> = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(receipts.pagination.total, 1);
        assert_eq!(receipts.items.len(), 1);
        if let PublicReceipt::Confirmed(receipt) = &receipts.items[0] {
            assert_eq!(receipt.ballot_id, 3);
        } else {
            panic!("Wrong receipt!");
        }
    }

    #[backend_test]
    async fn get_election_question_ballot(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let ballot = Coll::<Ballot<Audited>>::from_db(&db)
            .find_one(doc! {"state": Audited}, None)
            .await
            .unwrap()
            .unwrap();

        let response = client
            .get(uri!(election_question_ballot(
                ballot.election_id,
                ballot.question_id,
                ballot.ballot_id
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipt: PublicReceipt = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(
            receipt,
            PublicReceipt::from_ballot(AnyBallot::Audited(ballot), &election)
        );
    }

    #[backend_test]
    async fn candidate_totals(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let mut election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();

        // Ensure we cannot get the totals on an in-progress election.
        let response = client
            .get(uri!(candidate_totals(election.id, q1.id,)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::NotFound);

        // Set the end time in the past.
        election.metadata.end_time = Utc::now() - chrono::Duration::seconds(1);
        let result = Coll::<Election>::from_db(&db)
            .replace_one(u32_id_filter(election.id), &election, None)
            .await
            .unwrap();
        assert_eq!(result.modified_count, 1);

        // Ensure we can get the totals on a finished election.
        let response = client
            .get(uri!(candidate_totals(election.id, q1.id,)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let totals: HashMap<CandidateId, CandidateTotalsDesc> =
            serde_json::from_str(&raw_response).unwrap();
        assert_eq!(totals.len(), QuestionSpec::example1().candidates.len());
    }

    #[backend_test]
    async fn question_dump(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let mut election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();

        // Try with an in-progress election.
        let response = client
            .get(uri!(question_dump(election.id, q1.id)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let results: ElectionResults = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(
            results.election,
            ElectionDescription::from(election.clone()).crypto
        );
        assert!(results.totals.is_none());
        assert!(results.verify().is_ok());

        // Try with a finished election.
        election.metadata.end_time = Utc::now() - chrono::Duration::seconds(1);
        let result = Coll::<Election>::from_db(&db)
            .replace_one(u32_id_filter(election.id), &election, None)
            .await
            .unwrap();
        assert_eq!(result.modified_count, 1);

        let response = client
            .get(uri!(question_dump(election.id, q1.id)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let results: ElectionResults = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(results.election, ElectionDescription::from(election).crypto);
        assert!(results.totals.is_some());
        assert!(results.verify().is_ok());
    }

    /// This isn't really a test, but a way of generating test data for end-to-end tests.
    #[backend_test(admin)]
    async fn generate_test_data(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        // Put the election in the past.
        let mut election = get_election_for_spec(&db, ElectionSpec::current_example()).await;
        election.metadata.end_time = Utc::now() - chrono::Duration::seconds(1);
        let result = Coll::<Election>::from_db(&db)
            .replace_one(u32_id_filter(election.id), &election, None)
            .await
            .unwrap();
        assert_eq!(result.modified_count, 1);
        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let election_json = serde_json::to_string(&election).unwrap();
        println!("{election_json}\n");

        for question in election.questions.values() {
            let ballots = Coll::<AnyBallot>::from_db(&db)
                .find(
                    doc! {"question_id": question.id, "state": {"$ne": Unconfirmed}},
                    None,
                )
                .await
                .unwrap()
                .try_collect::<Vec<_>>()
                .await
                .unwrap();
            for ballot in ballots {
                let ballot_json = serde_json::to_string(&ballot).unwrap();
                println!("{ballot_json},");
            }
            println!();
            let totals = Coll::<CandidateTotals>::from_db(&db)
                .find(doc! {"question_id": question.id}, None)
                .await
                .unwrap()
                .try_collect::<Vec<_>>()
                .await
                .unwrap();
            for total in totals {
                let totals_json = serde_json::to_string(&total).unwrap();
                println!("{totals_json},");
            }
            println!("\nFull dump:");
            let response = client
                .get(uri!(question_dump(election.id, question.id)))
                .dispatch()
                .await;
            assert_eq!(response.status(), Status::Ok);
            let dump = response.into_string().await.unwrap();
            println!("{}", dump);
            println!("\n\n\n");
        }
    }

    async fn insert_elections(db: &Database) {
        Coll::<Election>::from_db(db)
            .insert_many(
                [
                    Election::published_example(),
                    Election::draft_example(),
                    Election::archived_example(),
                ],
                None,
            )
            .await
            .unwrap();
    }

    // Clippy doesn't like the ballot!() macros inside the vec![] macro, since
    // the order of resolving the ballot ID increments depends on the order of
    // evaluating the vector elements. It's fine though - the order doesn't matter.
    #[allow(clippy::eval_order_dependence)]
    async fn insert_ballots(db: &Database) {
        let election = get_election_for_spec(db, ElectionSpec::current_example()).await;
        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();
        let q1c1 = q1.candidates.get(0).unwrap();
        let q1c2 = q1.candidates.get(1).unwrap();
        let q2 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example2().description)
            .unwrap();
        let q2c1 = q2.candidates.get(0).unwrap();
        let q2c2 = q2.candidates.get(1).unwrap();
        let mut rng = rand::thread_rng();

        let mut candidate_totals = Vec::new();
        for candidate in q1.candidates.iter() {
            candidate_totals.push(NewCandidateTotals::new(
                election.id,
                q1.id,
                candidate.clone(),
            ));
        }
        for candidate in q2.candidates.iter() {
            candidate_totals.push(NewCandidateTotals::new(
                election.id,
                q2.id,
                candidate.clone(),
            ));
        }
        // This relies on no duplicate candidate names between questions, which is true for the examples.
        let mut totals_map = candidate_totals
            .iter_mut()
            .map(|t| (t.candidate_name.clone(), &mut t.crypto))
            .collect::<HashMap<_, _>>();

        let mut next_ballot_id = 1;
        macro_rules! ballot {
            ($q:ident, $yes:ident, $no:ident) => {{
                next_ballot_id += 1;
                BallotCore::new(
                    next_ballot_id,
                    $q.id,
                    $yes.clone(),
                    vec![$no.clone()],
                    &election,
                    &mut rng,
                )
                .unwrap()
            }};
        }

        // Create confirmed ballots.
        let confirmed = vec![
            // q1: 3 votes for candidate 1, 2 votes for candidate 2
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c1, q1c2).confirm(&mut totals_map),
            ballot!(q1, q1c2, q1c1).confirm(&mut totals_map),
            ballot!(q1, q1c2, q1c1).confirm(&mut totals_map),
            // q2: 3 votes for candidate 2
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
            ballot!(q2, q2c2, q2c1).confirm(&mut totals_map),
        ];

        // Create audited ballots.
        let audited = vec![
            // q1: 1 vote for each
            ballot!(q1, q1c1, q1c2).audit(),
            ballot!(q1, q1c2, q1c1).audit(),
            // q2: 3 votes for candidate 2, 1 vote for candidate 1
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c2, q2c1).audit(),
            ballot!(q2, q2c1, q2c2).audit(),
        ];

        // Create unconfirmed ballots.
        let unconfirmed = vec![
            // q1: 1 vote for each
            ballot!(q1, q1c1, q1c2),
            ballot!(q1, q1c2, q1c1),
            // q2: 1 vote for candidate 1
            ballot!(q2, q2c1, q2c2),
        ];

        // Insert ballots.
        Coll::<BallotCore<Confirmed>>::from_db(db)
            .insert_many(confirmed, None)
            .await
            .unwrap();
        Coll::<BallotCore<Audited>>::from_db(db)
            .insert_many(audited, None)
            .await
            .unwrap();
        Coll::<BallotCore<Unconfirmed>>::from_db(db)
            .insert_many(unconfirmed, None)
            .await
            .unwrap();
        // Insert candidate totals.
        Coll::<NewCandidateTotals>::from_db(db)
            .insert_many(candidate_totals, None)
            .await
            .unwrap();
    }

    async fn get_election_for_spec(db: &Database, election: ElectionSpec) -> Election {
        Coll::<Election>::from_db(db)
            .find_one(doc! { "name": &election.name }, None)
            .await
            .unwrap()
            .unwrap()
    }

    /// Dump the current state of the database to stdout; useful for debugging.
    #[allow(dead_code)]
    async fn dump_db_state(db: &Database) {
        println!("\nElections:");
        let mut elections = Coll::<Election>::from_db(db)
            .find(None, None)
            .await
            .unwrap();
        while let Some(Ok(election)) = elections.next().await {
            println!("{:#?}", election);
        }

        println!("\nCandidate Totals:");
        let mut totals = Coll::<CandidateTotals>::from_db(db)
            .find(None, None)
            .await
            .unwrap();
        while let Some(Ok(total)) = totals.next().await {
            println!("{:#?}", total);
        }
    }
}
