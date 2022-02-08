use mongodb::{
    bson::{doc, Document},
    options::FindOptions,
};
use rocket::{futures::TryStreamExt, http::Status, Route, serde::json::Json};
use serde::Serialize;

use crate::{
    error::{Error, Result},
    model::{
        admin::Admin,
        auth::AuthToken,
        ballot::{DbBallot, FinishedBallot, AUDITED, CONFIRMED},
        election::{Election, ElectionMetadata},
        mongodb::{Coll, Id},
        pagination::{PaginationRequest, PaginationResponse},
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
        .find_one(
            doc! {
                "_id": *election_id,
            },
            None,
        )
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!("No election found with ID `{}`", *election_id),
            )
        })?;
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
            Error::Status(
                Status::NotFound,
                format!("No finalised election found with ID `{}`", *election_id),
            )
        })?;

    Ok(Json(election))
}

#[get("/elections/<election_id>/<question_id>/ballots?<pagination..>")]
async fn election_question_ballots(
    election_id: Id,
    question_id: Id,
    pagination: PaginationRequest,
    ballots: Coll<FinishedBallot>,
) -> Result<Json<PaginatedBallots<FinishedBallot>>> {
    let confirmed_ballots_for_election_question = doc! {
        "election_id": *election_id,
        "question_id": *question_id,
        "$or": [{"state": AUDITED}, {"state": CONFIRMED}],
    };

    let pagination_options = FindOptions::builder()
        .skip(u64::from(pagination.skip()))
        .limit(i64::from(pagination.page_size()))
        .build();

    let ballots = ballots
        .find(confirmed_ballots_for_election_question, pagination_options)
        .await?
        .try_collect::<Vec<_>>()
        .await?;

    let pagination_result = pagination.into_response(ballots.len());

    Ok(Json(PaginatedBallots {
        ballots,
        pagination_result,
    }))
}

#[get("/elections/<election_id>/<question_id>/ballots/<ballot_id>")]
async fn election_question_ballot(
    election_id: Id,
    question_id: Id,
    ballot_id: Id,
    ballots: Coll<FinishedBallot>,
) -> Result<Option<Json<FinishedBallot>>> {
    let election_question_ballot = doc! {
        "_id": *ballot_id,
        "election_id": *election_id,
        "question_id": *question_id,
        "$or": [{"state": AUDITED}, {"state": CONFIRMED}],
    };

    let ballot = ballots
        .find_one(election_question_ballot, None)
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!(
                    "a ballot with ID `{:?}` for election {:?}, question {:?} does not exist",
                    ballot_id, election_id, question_id
                ),
            )
        })?;

    Ok(Some(Json(ballot)))
}

async fn elections_matching(
    elections: Coll<ElectionMetadata>,
    filter: impl Into<Option<Document>>,
) -> Result<Json<Vec<ElectionMetadata>>> {
    let elections = elections.find(filter, None).await?.try_collect().await?;
    Ok(Json(elections))
}

#[derive(Serialize)]
pub struct PaginatedBallots<B: DbBallot> {
    ballots: Vec<B>,
    #[serde(flatten)]
    pagination_result: PaginationResponse,
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{local::asynchronous::Client, serde::json::serde_json};

    use crate::model::{
        election::{Election, ElectionMetadata, ElectionSpec, NewElection},
        mongodb::DbEntity,
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
            .get(uri!(election(finalised_election.id())))
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
            .get(uri!(election(unfinalised_election.id())))
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
            .get(uri!(finalised_election(finalised_election.id())))
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
            .get(uri!(finalised_election(unfinalised_election.id())))
            .dispatch()
            .await;

        assert_eq!(Status::NotFound, response.status());
    }

    #[backend_test]
    async fn get_election_strips_vote_data() {
        // TODO Ask CR if this should really happen
    }

    // #[backend_test]
    // async fn get_election_question_ballots_as_admin() {
    //     let (client, db) = client_and_db().await;

    //     let response = client.get(uri!(election_question_ballots_for_admin("")));
    // }

    // #[backend_test]
    // async fn get_finalised_election_question_ballots_as_non_admin() {
    //     let (client, db) = client_and_db().await;

    //     let response = client
    //         .get(uri!(election_question_ballots_for_non_admin(
    //             "61edd4d941984f862fd14a6f".parse::<Id>().unwrap(),
    //             0,
    //             Pagination::new(1, 1)
    //         )))
    //         .dispatch()
    //         .await;

    //     assert_eq!(Status::Ok, response.status());

    //     let raw_response = response.into_string().await.unwrap();

    //     let ballots = serde_json::from_str::<Vec<Ballot>>(&raw_response).unwrap();

    //     assert_eq!(vec![Ballot::new(vec![], Pwf, State::Confirmed)], ballots);
    // }

    // #[backend_test]
    // async fn fail_to_get_unfinalised_election_question_ballots_as_non_admin() {
    //     let (client, db) = client_and_db().await;

    //     let response = client
    //         .get(uri!(election_question_ballots_for_non_admin(
    //             "".parse::<Id>().unwrap(),
    //             0,
    //             Pagination::new(1, 1)
    //         )))
    //         .dispatch()
    //         .await;

    //     assert_eq!(Status::NotFound, response.status());
    // }

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

    async fn get_election_for_spec(db: &Database, election: ElectionSpec) -> Election {
        Coll::<Election>::from_db(&db)
            .find_one(doc! { "name": &election.metadata.name }, None)
            .await
            .unwrap()
            .unwrap()
    }
}
