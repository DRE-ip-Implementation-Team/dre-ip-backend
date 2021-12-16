use crate::{
    error::{Error, Result},
    model::{
        admin::Admin,
        auth::token::Token,
        bson::Id,
        election::{Ballot, Election, Elections},
        pagination::{Metadata, Pagination},
        voter::Voter,
    },
};

use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::{FindOneOptions, FindOptions},
};
use rocket::{serde::json::Json, Route, State};
use serde::Serialize;

pub fn routes() -> Vec<Route> {
    routes![
        get_elections_admin,
        get_elections_voter,
        get_election_admin,
        get_election_voter,
        get_votes,
        get_vote
    ]
}

#[get("/elections", rank = 1)]
async fn get_elections_admin(
    _token: Token<Admin>,
    elections: &State<Elections>,
) -> Result<Json<Vec<Election>>> {
    get_elections_with_filter(elections, None).await
}

#[get("/elections", rank = 2)]
async fn get_elections_voter(
    _token: Token<Voter>,
    elections: &State<Elections>,
) -> Result<Json<Vec<Election>>> {
    get_elections_with_filter(
        elections,
        doc! {
            "finalised": true
        },
    )
    .await
}

#[get("/elections/<election_id>", rank = 1)]
async fn get_election_admin(
    _token: Token<Admin>,
    election_id: Id,
    elections: &State<Elections>,
) -> Result<Json<Election>> {
    get_election_with_filter(elections, election_id, None).await
}

#[get("/elections/<election_id>", rank = 2)]
async fn get_election_voter(
    _token: Token<Voter>,
    election_id: Id,
    elections: &State<Elections>,
) -> Result<Json<Election>> {
    get_election_with_filter(
        elections,
        election_id,
        doc! {
            "finalised": true
        },
    )
    .await
}

#[get("/elections/<election_id>/votes?<pagination..>")]
async fn get_votes(
    election_id: Id,
    pagination: Pagination,
    elections: &State<Elections>,
) -> Result<Json<PaginatedBallots>> {
    let ballots = elections
        .find_one(
            doc! { "_id": *election_id },
            FindOneOptions::builder()
                .projection(doc! {
                    "ballots": {
                        "$slice": [
                            pagination.skip(),
                            pagination.page_size(),
                        ]
                    }
                })
                .build(),
        )
        .await?
        .ok_or_else(|| {
            Error::NotFound(format!(
                "an election with ID `{:?}` does not exist",
                election_id
            ))
        })?
        .ballots();
    let pagination_result = pagination.into_metadata(ballots.len());
    Ok(Json(PaginatedBallots {
        ballots,
        pagination_result,
    }))
}

#[get("/elections/<election_id>/votes/<vote_id>")]
async fn get_vote(
    election_id: Id,
    vote_id: Id,
    elections: &State<Elections>,
) -> Result<Option<Json<Ballot>>> {
    Ok(elections
        .find_one(
            doc! { "_id": *election_id },
            FindOneOptions::builder()
                .projection(doc! {
                    "ballots": {
                        "_id": *vote_id,
                    }
                })
                .build(),
        )
        .await?
        .ok_or_else(|| {
            Error::NotFound(format!(
                "an election with ID `{:?}` does not exist",
                election_id
            ))
        })?
        .ballots()
        .first()
        .map(|ballot| Json(ballot.clone())))
}

async fn get_elections_with_filter(
    elections: &State<Elections>,
    filter: impl Into<Option<Document>>,
) -> Result<Json<Vec<Election>>> {
    Ok(Json(
        elections
            .find(
                filter,
                FindOptions::builder()
                    .projection(doc! {
                        "ballots": {
                            "$slice": 0 // Creates an empty Vec
                        },
                    })
                    .build(),
            )
            .await?
            .try_collect()
            .await?,
    ))
}

async fn get_election_with_filter(
    elections: &State<Elections>,
    id: Id,
    filter: impl Into<Option<Document>>,
) -> Result<Json<Election>> {
    Ok(Json(elections.find_one(filter, None).await?.ok_or_else(
        || Error::NotFound(format!("Could not find an election with ID `{}`", *id)),
    )?))
}

#[derive(Serialize)]
pub struct PaginatedBallots {
    ballots: Vec<Ballot>,
    #[serde(flatten)]
    pagination_result: Metadata,
}
