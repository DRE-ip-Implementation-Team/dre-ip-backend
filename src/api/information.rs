use crate::{
    error::{Error, Result},
    model::{
        bson::Id,
        election::{Ballot, Election, Elections},
        pagination::{Metadata, Pagination},
    },
};

use futures::stream::TryStreamExt;
use mongodb::{
    bson::doc,
    options::{FindOneOptions, FindOptions},
    Collection,
};
use rocket::{serde::json::Json, Route, State};
use serde::Serialize;

pub fn routes() -> Vec<Route> {
    routes![get_elections, get_votes, get_vote]
}

#[get("/elections")]
async fn get_elections(elections: &State<Collection<Election>>) -> Result<Json<Vec<Election>>> {
    Ok(Json(
        elections
            .find(
                None,
                FindOptions::builder()
                    .projection(doc! {
                        "ballots": {
                            "$slice": [0, 0] // Creates an empty Vec
                        },
                    })
                    .build(),
            )
            .await?
            .try_collect()
            .await?,
    ))
}

#[get("/election/<election_id>/votes?<pagination..>")]
async fn get_votes(
    election_id: Id,
    pagination: Pagination,
    elections: &State<Elections>,
) -> Result<Json<PaginatedBallots>> {
    let ballots = elections
        .find_one(
            doc! { "electionId": &election_id },
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

#[get("/election/<election_id>/votes/<vote_id>")]
async fn get_vote(
    election_id: Id,
    vote_id: Id,
    elections: &State<Elections>,
) -> Result<Option<Json<Ballot>>> {
    Ok(elections
        .find_one(
            doc! { "_id": &election_id },
            FindOneOptions::builder()
                .projection(doc! {
                    "ballots": {
                        "_id": &vote_id,
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

#[derive(Serialize)]
pub struct PaginatedBallots {
    ballots: Vec<Ballot>,
    #[serde(flatten)]
    pagination_result: Metadata,
}
