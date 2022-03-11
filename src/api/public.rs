use std::collections::HashMap;

use dre_ip::ElectionResults;
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
use serde::{Deserialize, Serialize};

use crate::{
    error::{Error, Result},
    model::{
        api::{
            auth::AuthToken,
            pagination::{Paginated, PaginationRequest},
        },
        base::{CandidateId, DreipGroup, ElectionMetadata, ElectionState},
        db::{
            Admin, Audited, CandidateTotals, Confirmed, ElectionNoSecrets, ElectionWithSecrets,
            FinishedBallot, FinishedReceipt,
        },
        mongodb::{Coll, Id},
    },
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
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<Vec<MetadataWithId>>> {
    let archived = archived.unwrap_or(false);
    metadata_for_elections(elections, true, archived).await
}

#[get("/elections?<archived>", rank = 2)]
async fn elections_non_admin(
    archived: Option<bool>,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<Vec<MetadataWithId>>> {
    let archived = archived.unwrap_or(false);
    metadata_for_elections(elections, false, archived).await
}

#[get("/elections/<election_id>", rank = 1)]
async fn election_admin(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<ElectionNoSecrets>> {
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;
    Ok(Json(election))
}

#[get("/elections/<election_id>", rank = 2)]
async fn election_non_admin(
    election_id: Id,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<ElectionNoSecrets>> {
    let filter = doc! {
        "_id": (election_id),
        "$or": [{"state": ElectionState::Published}, {"state": ElectionState::Archived}],
    };

    let election = elections
        .find_one(filter, None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Non-admin election with ID '{}'", election_id)))?;

    Ok(Json(election))
}

#[get("/elections/<election_id>/<question_id>/ballots?<pagination..>")]
async fn election_question_ballots(
    election_id: Id,
    question_id: Id,
    pagination: PaginationRequest,
    elections: Coll<ElectionWithSecrets>,
    ballots: Coll<FinishedBallot>,
) -> Result<Json<Paginated<FinishedReceipt>>> {
    // No need to filter our drafts if non-admin, since draft elections cannot have ballots.
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let filter = doc! {
        "election_id": (election_id),
        "question_id": (question_id),
        "$or": [{"state": Audited}, {"state": Confirmed}],
    };

    let pagination_options = FindOptions::builder()
        .skip(u64::from(pagination.skip()))
        .limit(i64::from(pagination.page_size()))
        .build();

    let ballots_page = ballots
        .find(filter.clone(), pagination_options)
        .await?
        .map(|ballot| ballot.map(|ballot| FinishedReceipt::from_finished_ballot(ballot, &election)))
        .try_collect::<Vec<_>>()
        .await?;

    let total_ballots = ballots.count_documents(filter, None).await?;

    let paginated = pagination.to_paginated(total_ballots, ballots_page);
    Ok(Json(paginated))
}

#[get("/elections/<election_id>/<question_id>/ballots/<ballot_id>")]
async fn election_question_ballot(
    election_id: Id,
    question_id: Id,
    ballot_id: Id,
    elections: Coll<ElectionWithSecrets>,
    ballots: Coll<FinishedBallot>,
) -> Result<Json<FinishedReceipt>> {
    // No need to filter our drafts if non-admin, since draft elections cannot have ballots.
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let election_question_ballot = doc! {
        "_id": (ballot_id),
        "election_id": (election_id),
        "question_id": (question_id),
        "$or": [{"state": Audited}, {"state": Confirmed}],
    };

    let ballot = ballots
        .find_one(election_question_ballot, None)
        .await?
        .map(|ballot| FinishedReceipt::from_finished_ballot(ballot, &election))
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
    election_id: Id,
    question_id: Id,
    totals: Coll<CandidateTotals>,
) -> Result<Json<HashMap<CandidateId, CandidateTotals>>> {
    // No need to filter our drafts if non-admin, since draft elections cannot have totals.
    let question_totals_filter = doc! {
        "election_id": (election_id),
        "question_id": (question_id),
    };
    let question_totals = totals
        .find(question_totals_filter, None)
        .await?
        .map_ok(|tot| (tot.candidate_name.clone(), tot))
        .try_collect::<HashMap<_, _>>()
        .await?;

    Ok(Json(question_totals))
}

#[get("/elections/<election_id>/<question_id>/dump")]
async fn question_dump(
    election_id: Id,
    question_id: Id,
    elections: Coll<ElectionNoSecrets>,
    totals: Coll<CandidateTotals>,
    ballots: Coll<FinishedBallot>,
    db_client: &State<Client>,
) -> Result<Json<ElectionResults<String, String, DreipGroup>>> {
    // Ensure we read a consistent snapshot of the election data.
    let election;
    let mut election_totals = HashMap::new();
    let mut audited_ballots = HashMap::new();
    let mut confirmed_ballots = HashMap::new();
    {
        let session_options = SessionOptions::builder().snapshot(true).build();
        let mut session = db_client.start_session(Some(session_options)).await?;

        let election_filter = doc! {
            "_id": (election_id),
            "$or": [{"state": ElectionState::Published}, {"state": ElectionState::Archived}],
        };
        election = elections
            .find_one_with_session(election_filter, None, &mut session)
            .await?
            .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

        let totals_filter = doc! {
            "election_id": (election_id),
            "question_id": (question_id),
        };
        let mut totals_cursor = totals
            .find_with_session(totals_filter, None, &mut session)
            .await?;
        while let Some(total) = totals_cursor.next(&mut session).await {
            let total = total?;
            election_totals.insert(total.candidate_name.clone(), total.totals.crypto);
        }

        let ballots_filter = doc! {
            "election_id": (election_id),
            "question_id": (question_id),
            "$or": [{"state": Audited}, {"state": Confirmed}],
        };
        let mut election_ballots = ballots
            .find_with_session(ballots_filter, None, &mut session)
            .await?;
        while let Some(ballot) = election_ballots.next(&mut session).await {
            match ballot? {
                FinishedBallot::Audited(b) => {
                    audited_ballots.insert(b.id.to_string(), b.ballot.crypto);
                }
                FinishedBallot::Confirmed(b) => {
                    confirmed_ballots.insert(b.id.to_string(), b.ballot.crypto);
                }
            }
        }
    }

    let dump = ElectionResults {
        election: election.election.crypto,
        audited: audited_ballots,
        confirmed: confirmed_ballots,
        totals: election_totals,
    };

    Ok(Json(dump))
}

/// Retrieve the metadata for elections.
/// If `admin` is false, admin-only elections will be hidden.
/// If `archived` is true, archived elections will be returned instead of non-archived ones.
async fn metadata_for_elections(
    elections: Coll<ElectionNoSecrets>,
    admin: bool,
    archived: bool,
) -> Result<Json<Vec<MetadataWithId>>> {
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

    let metadata = elections
        .into_iter()
        .map(|e| MetadataWithId {
            id: e.id,
            meta: e.election.metadata,
        })
        .collect();

    Ok(Json(metadata))
}

/// A simple struct that tacks an ID onto the election metadata.
#[derive(Debug, Serialize, Deserialize)]
struct MetadataWithId {
    pub id: Id,
    #[serde(flatten)]
    pub meta: ElectionMetadata,
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json};
    use std::collections::HashMap;

    use crate::model::{
        base::{ElectionSpec, QuestionSpec},
        db::{Ballot, NewCandidateTotals, NewElection, Unconfirmed},
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
        let fetched_elections = serde_json::from_str::<Vec<MetadataWithId>>(&raw_response)
            .unwrap()
            .into_iter()
            .map(|m| m.meta)
            .collect::<Vec<_>>();

        let expected = vec![
            NewElection::published_example().metadata,
            NewElection::draft_example().metadata,
        ];

        assert_eq!(expected, fetched_elections);
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

        let fetched_elections = serde_json::from_str::<Vec<MetadataWithId>>(&raw_response)
            .unwrap()
            .into_iter()
            .map(|m| m.meta)
            .collect::<Vec<_>>();

        let elections = vec![NewElection::published_example().metadata];

        assert_eq!(elections, fetched_elections);
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
        let error = serde_json::from_str::<ElectionWithSecrets>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionNoSecrets>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = NewElection::published_example().erase_secrets();

        assert_eq!(expected.metadata, fetched_election.metadata);
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
        let error = serde_json::from_str::<ElectionWithSecrets>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionNoSecrets>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = NewElection::draft_example().erase_secrets();

        assert_eq!(expected.metadata, fetched_election.metadata);
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
        let error = serde_json::from_str::<ElectionWithSecrets>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionNoSecrets>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = NewElection::published_example().erase_secrets();

        assert_eq!(expected.metadata, fetched_election.metadata);
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
        let fetched_elections = serde_json::from_str::<Vec<MetadataWithId>>(&raw_response)
            .unwrap()
            .into_iter()
            .map(|m| m.meta)
            .collect::<Vec<_>>();

        let expected = vec![NewElection::archived_example().metadata];
        assert_eq!(expected, fetched_elections);

        // Try getting a specific archived election.
        let election_id = serde_json::from_str::<Vec<MetadataWithId>>(&raw_response)
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
        let error = serde_json::from_str::<ElectionWithSecrets>(&raw_response);
        assert!(error.is_err());
        let fetched_election = serde_json::from_str::<ElectionNoSecrets>(&raw_response).unwrap();

        // Note: the IDs and crypto will be different here so we need to be careful with comparisons.
        let expected = NewElection::archived_example().erase_secrets();

        assert_eq!(expected.metadata, fetched_election.metadata);
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
                pagination
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipts: Paginated<FinishedReceipt> = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(receipts.pagination.page_num, 1);
        assert_eq!(receipts.pagination.page_size, 2);
        assert_eq!(receipts.pagination.total, 7);
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
                pagination
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipts: Paginated<FinishedReceipt> = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(receipts.pagination.page_num, 1);
        assert_eq!(receipts.pagination.page_size, 50);
        assert_eq!(receipts.pagination.total, 7);
        assert_eq!(receipts.items.len(), 7);
        assert_eq!(
            receipts
                .items
                .iter()
                .filter(|receipt| {
                    match receipt {
                        FinishedReceipt::Audited(_) => false,
                        FinishedReceipt::Confirmed(_) => true,
                    }
                })
                .count(),
            5
        );
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
                ballot.id
            )))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let receipt: FinishedReceipt = serde_json::from_str(&raw_response).unwrap();
        assert_eq!(
            receipt,
            FinishedReceipt::from_finished_ballot(FinishedBallot::Audited(ballot), &election)
        );
    }

    #[backend_test]
    async fn candidate_totals(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();

        let response = client
            .get(uri!(candidate_totals(election.id, q1.id,)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let totals: HashMap<CandidateId, CandidateTotals> =
            serde_json::from_str(&raw_response).unwrap();
        assert_eq!(totals.len(), QuestionSpec::example1().candidates.len());
    }

    #[backend_test]
    async fn question_dump(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let q1 = election
            .questions
            .values()
            .find(|q| q.description == QuestionSpec::example1().description)
            .unwrap();

        let response = client
            .get(uri!(question_dump(election.id, q1.id,)))
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let results: ElectionResults<Id, CandidateId, DreipGroup> =
            serde_json::from_str(&raw_response).unwrap();
        // Id does not and cannot implement AsRef<[u8]>, hence the awkward workaround.
        let results = ElectionResults {
            election: results.election,
            audited: results
                .audited
                .into_iter()
                .map(|(id, b)| (id.to_bytes(), b))
                .collect(),
            confirmed: results
                .confirmed
                .into_iter()
                .map(|(id, b)| (id.to_bytes(), b))
                .collect(),
            totals: results.totals,
        };
        assert_eq!(results.election, election.erase_secrets().crypto);
        assert!(results.verify().is_ok());
    }

    /// This isn't really a test, but a way of generating test data for end-to-end tests.
    #[backend_test(admin)]
    async fn generate_test_data(db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let election = get_election_for_spec(&db, ElectionSpec::current_example()).await;

        let election_json = serde_json::to_string(&election).unwrap();
        println!("{election_json}\n");

        for question in election.questions.values() {
            let ballots = Coll::<FinishedBallot>::from_db(&db)
                .find(
                    doc! {"question_id": *question.id, "state": {"$ne": Unconfirmed}},
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
                .find(doc! {"question_id": *question.id}, None)
                .await
                .unwrap()
                .try_collect::<Vec<_>>()
                .await
                .unwrap();
            for total in totals {
                let totals_json = serde_json::to_string(&total).unwrap();
                println!("{totals_json},");
            }
            println!("\n\n\n");
        }
    }

    async fn insert_elections(db: &Database) {
        Coll::<NewElection>::from_db(&db)
            .insert_many(
                [
                    NewElection::published_example(),
                    NewElection::draft_example(),
                    NewElection::archived_example(),
                ],
                None,
            )
            .await
            .unwrap();
    }

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

        macro_rules! ballot {
            ($q:ident, $yes:ident, $no:ident) => {
                Ballot::new($q.id, $yes.clone(), vec![$no.clone()], &election, &mut rng).unwrap()
            };
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
        Coll::<Ballot<Confirmed>>::from_db(db)
            .insert_many(confirmed, None)
            .await
            .unwrap();
        Coll::<Ballot<Audited>>::from_db(db)
            .insert_many(audited, None)
            .await
            .unwrap();
        Coll::<Ballot<Unconfirmed>>::from_db(db)
            .insert_many(unconfirmed, None)
            .await
            .unwrap();
        // Insert candidate totals.
        Coll::<NewCandidateTotals>::from_db(db)
            .insert_many(candidate_totals, None)
            .await
            .unwrap();
    }

    async fn get_election_for_spec(db: &Database, election: ElectionSpec) -> ElectionWithSecrets {
        Coll::<ElectionWithSecrets>::from_db(&db)
            .find_one(doc! { "name": &election.name }, None)
            .await
            .unwrap()
            .unwrap()
    }

    /// Dump the current state of the database to stdout; useful for debugging.
    #[allow(dead_code)]
    async fn dump_db_state(db: &Database) {
        println!("\nElections:");
        let mut elections = Coll::<ElectionWithSecrets>::from_db(db)
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
