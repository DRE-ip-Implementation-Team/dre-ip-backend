use crate::{
    error::Error,
    model::election::{Ballot, Election},
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

#[get("/election/<election_id>/ballots?<page_num>&<page_size>")]
async fn get_ballots(
    election_id: Id,
    page_num: Option<usize>,
    page_size: Option<usize>,
    ballots: &State<Collection<Ballot>>,
) -> Result<Json<PaginatedBallots>, Error> {
    let page_num = page_num.map(|n| n - 1).unwrap_or(0);
    let page_size = page_size.unwrap_or(50);
    let ballots = ballots
        .find(
            doc! { "electionId": election_id },
            FindOptions::builder()
                .skip((page_num * page_size) as u64)
                .limit(page_size as i64)
                .batch_size(page_size as u32)
                .build(),
        )
        .await?
        .try_collect()
        .await?;
    let pagination = Pagination::new(page_num, page_size, 3000);
    Ok(Json(PaginatedBallots {
        ballots,
        pagination,
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
    pagination: Pagination,
}

#[derive(Serialize)]
pub struct Pagination {
    page_num: usize,
    page_size: usize,
    total: usize,
}

impl Pagination {
    fn new(page_num: usize, page_size: usize, total: usize) -> Self {
        Self {
            page_num,
            page_size,
            total,
        }
    }
}
