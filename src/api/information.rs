use crate::{
    error::Error,
    model::{
        election::{Ballot, Election},
        pagination::{Pagination, PaginationResult},
    },
};
use futures::stream::TryStreamExt;
use mongodb::{
    bson::{doc, oid::ObjectId, Bson},
    options::FindOptions,
    Collection,
};
use rocket::{request::FromParam, serde::json::Json, Route, State};
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

#[get("/election/<election_id>/ballots?<_page_size>&<_page_num>")]
async fn get_ballots(
    election_id: Id,
    _page_size: Option<usize>,
    _page_num: Option<usize>,
    pagination: Pagination,
    ballots: &State<Collection<Ballot>>,
) -> Result<Json<PaginatedBallots>, Error> {
    // TODO: Tell if query found nothing?
    let ballots = ballots
        .find(
            doc! { "electionId": election_id },
            FindOptions::builder()
                .skip(pagination.skip())
                .limit(pagination.page_size() as i64)
                .batch_size(pagination.page_size() as u32)
                .build(),
        )
        .await?
        .try_collect::<Vec<Ballot>>()
        .await?;
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
        .find_one(doc! { "_id": ballot_id }, None)
        .await?
        .map(Json))
}

pub struct Id(ObjectId);

impl From<Id> for Bson {
    fn from(id: Id) -> Self {
        id.0.into()
    }
}

impl<'a> FromParam<'a> for Id {
    type Error = mongodb::bson::oid::Error;

    fn from_param(param: &'a str) -> Result<Self, Self::Error> {
        Ok(Self(param.parse::<ObjectId>()?))
    }
}

#[derive(Serialize)]
pub struct PaginatedBallots {
    ballots: Vec<Ballot>,
    #[serde(flatten)]
    pagination_result: PaginationResult,
}
