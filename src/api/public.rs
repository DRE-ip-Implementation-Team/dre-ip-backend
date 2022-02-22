use mongodb::{
    bson::{doc, Document},
    options::FindOptions,
};
use rocket::{
    futures::{StreamExt, TryStreamExt},
    serde::json::Json,
    Route,
};

use crate::{
    error::{Error, Result},
    model::{
        admin::Admin,
        auth::AuthToken,
        ballot::{FinishedBallot, FinishedReceipt, AUDITED, CONFIRMED},
        election::{Election, ElectionMetadata},
        mongodb::{Coll, Id},
        pagination::{Paginated, PaginationRequest},
    },
};

pub fn routes() -> Vec<Route> {
    routes![
        elections,
        finalised_elections,
        election,
        finalised_election,
        election_question_ballots,
        election_question_ballot
    ]
}

#[get("/elections", rank = 1)]
async fn elections(
    _token: AuthToken<Admin>,
    elections: Coll<ElectionMetadata>,
) -> Result<Json<Vec<ElectionMetadata>>> {
    elections_matching(elections, None).await
}

#[get("/elections", rank = 2)]
async fn finalised_elections(
    elections: Coll<ElectionMetadata>,
) -> Result<Json<Vec<ElectionMetadata>>> {
    elections_matching(
        elections,
        doc! {
            "finalised": true
        },
    )
    .await
}

#[get("/elections/<election_id>", rank = 1)]
async fn election(
    _token: AuthToken<Admin>,
    election_id: Id,
    elections: Coll<Election>,
) -> Result<Json<Election>> {
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", *election_id)))?;
    Ok(Json(election))
}

#[get("/elections/<election_id>", rank = 2)]
async fn finalised_election(election_id: Id, elections: Coll<Election>) -> Result<Json<Election>> {
    let finalised_election = doc! {
        "_id": *election_id,
        "finalised": true,
    };

    let election = elections
        .find_one(finalised_election, None)
        .await?
        .ok_or_else(|| {
            Error::not_found(format!("Finalised election with ID '{}'", *election_id))
        })?;

    Ok(Json(election))
}

#[get("/elections/<election_id>/<question_id>/ballots?<pagination..>")]
async fn election_question_ballots(
    election_id: Id,
    question_id: Id,
    pagination: PaginationRequest,
    elections: Coll<Election>,
    ballots: Coll<FinishedBallot>,
) -> Result<Json<Paginated<FinishedReceipt>>> {
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let filter = doc! {
        "election_id": *election_id,
        "question_id": *question_id,
        "$or": [{"state": AUDITED}, {"state": CONFIRMED}],
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
    elections: Coll<Election>,
    ballots: Coll<FinishedBallot>,
) -> Result<Json<FinishedReceipt>> {
    let election = elections
        .find_one(election_id.as_doc(), None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))?;

    let election_question_ballot = doc! {
        "_id": *ballot_id,
        "election_id": *election_id,
        "question_id": *question_id,
        "$or": [{"state": AUDITED}, {"state": CONFIRMED}],
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

// TODO get candidate totals

// TODO get entire election dump

async fn elections_matching(
    elections: Coll<ElectionMetadata>,
    filter: impl Into<Option<Document>>,
) -> Result<Json<Vec<ElectionMetadata>>> {
    let elections = elections.find(filter, None).await?.try_collect().await?;
    Ok(Json(elections))
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{http::Status, local::asynchronous::Client, serde::json::serde_json};
    use std::collections::HashMap;

    use crate::model::{
        ballot::{Audited, Ballot, Confirmed, Unconfirmed},
        candidate_totals::NewCandidateTotals,
        election::{Election, ElectionMetadata, ElectionSpec, NewElection, QuestionSpec},
    };

    use super::*;

    #[backend_test(admin)]
    async fn get_all_elections_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let response = client.get(uri!(elections)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();
        let fetched_elections =
            serde_json::from_str::<Vec<ElectionMetadata>>(&raw_response).unwrap();

        let expected = vec![
            ElectionMetadata::from(ElectionSpec::finalised_example()),
            ElectionMetadata::from(ElectionSpec::unfinalised_example()),
        ];

        assert_eq!(expected, fetched_elections);
    }

    #[backend_test]
    async fn get_finalised_elections_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let response = client.get(uri!(finalised_elections)).dispatch().await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        let fetched_elections =
            serde_json::from_str::<Vec<ElectionMetadata>>(&raw_response).unwrap();

        let elections = vec![ElectionMetadata::from(ElectionSpec::finalised_example())];

        assert_eq!(elections, fetched_elections);
    }

    #[backend_test(admin)]
    async fn get_finalised_election_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let finalised_election =
            get_election_for_spec(&db, ElectionSpec::finalised_example()).await;

        let response = client
            .get(uri!(election(finalised_election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        let fetched_election = serde_json::from_str::<ElectionMetadata>(&raw_response).unwrap();

        let expected = ElectionMetadata::from(ElectionSpec::finalised_example());

        assert_eq!(expected, fetched_election);
    }

    #[backend_test(admin)]
    async fn get_unfinalised_election_as_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let unfinalised_election =
            get_election_for_spec(&db, ElectionSpec::unfinalised_example()).await;

        let response = client
            .get(uri!(election(unfinalised_election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        let fetched_election = serde_json::from_str::<ElectionMetadata>(&raw_response).unwrap();

        let expected = ElectionMetadata::from(ElectionSpec::unfinalised_example());

        assert_eq!(expected, fetched_election);
    }

    #[backend_test]
    async fn get_finalised_election_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let finalised_election =
            get_election_for_spec(&db, ElectionSpec::finalised_example()).await;

        let response = client
            .get(uri!(finalised_election(finalised_election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());
        assert!(response.body().is_some());

        let raw_response = response.into_string().await.unwrap();

        let fetched_election = serde_json::from_str::<ElectionMetadata>(&raw_response).unwrap();

        let expected = ElectionMetadata::from(ElectionSpec::finalised_example());

        assert_eq!(expected, fetched_election);
    }

    #[backend_test]
    async fn fail_to_get_unfinalised_election_as_non_admin(client: Client, db: Database) {
        insert_elections(&db).await;

        let unfinalised_election =
            get_election_for_spec(&db, ElectionSpec::unfinalised_example()).await;

        let response = client
            .get(uri!(finalised_election(unfinalised_election.id)))
            .dispatch()
            .await;

        assert_eq!(Status::NotFound, response.status());
    }

    #[backend_test]
    async fn get_election_question_ballots(client: Client, db: Database) {
        insert_elections(&db).await;
        insert_ballots(&db).await;

        let finalised = get_election_for_spec(&db, ElectionSpec::finalised_example()).await;
        let question_id = *finalised
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
                finalised.id,
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
                finalised.id,
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

        let finalised = get_election_for_spec(&db, ElectionSpec::finalised_example()).await;

        let ballot = Coll::<Ballot<Audited>>::from_db(&db)
            .find_one(doc! {"state": AUDITED}, None)
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
            FinishedReceipt::from_finished_ballot(FinishedBallot::Audited(ballot), &finalised)
        );
    }

    async fn insert_elections(db: &Database) {
        Coll::<NewElection>::from_db(&db)
            .insert_many(
                [
                    NewElection::from(ElectionSpec::finalised_example()),
                    NewElection::from(ElectionSpec::unfinalised_example()),
                ],
                None,
            )
            .await
            .unwrap();
    }

    async fn insert_ballots(db: &Database) {
        let election = get_election_for_spec(db, ElectionSpec::finalised_example()).await;
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

    async fn get_election_for_spec(db: &Database, election: ElectionSpec) -> Election {
        Coll::<Election>::from_db(&db)
            .find_one(doc! { "name": &election.metadata.name }, None)
            .await
            .unwrap()
            .unwrap()
    }
}
