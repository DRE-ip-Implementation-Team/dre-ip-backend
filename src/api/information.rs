use crate::{
    error::Error,
    model::{
        bson::Id,
        election::{Ballot, Election},
        pagination::{Pagination, PaginationResult},
    },
};
use futures::stream::TryStreamExt;
use mongodb::{bson::doc, options::FindOneOptions, Collection};
use rocket::{serde::json::Json, Route, State};
use serde::Serialize;

pub fn routes() -> Vec<Route> {
    routes![get_elections, get_ballots, get_ballot]
}

#[get("/elections")]
async fn get_elections(
    elections: &State<Collection<Election>>,
) -> Result<Json<Vec<Election>>, Error> {
    Ok(Json(elections.find(None, None).await?.try_collect().await?))
}

#[get("/election/<election_id>/ballots?<pagination..>")]
async fn get_ballots(
    election_id: Id,
    pagination: Pagination,
    elections: &State<Collection<Election>>,
) -> Result<Json<PaginatedBallots>, Error> {
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
        .ok_or(Error::NotFound(format!(
            "an election with ID `{:?}` does not exist",
            election_id
        )))?
        .ballots();
    let pagination_result = pagination.result(ballots.len());
    Ok(Json(PaginatedBallots {
        ballots,
        pagination_result,
    }))
}

#[get("/election/<ballot_id>")]
async fn get_ballot(
    ballot_id: Id,
    ballots: &State<Collection<Ballot>>,
) -> Result<Option<Json<Ballot>>, Error> {
    Ok(ballots
        .find_one(doc! { "_id": &ballot_id }, None)
        .await?
        .map(Json))
}

#[derive(Serialize)]
pub struct PaginatedBallots {
    ballots: Vec<Ballot>,
    #[serde(flatten)]
    pagination_result: PaginationResult,
}
