use chrono::Utc;
use mongodb::bson::doc;
use rocket::http::Status;

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    election::Election,
    mongodb::{Coll, Id},
    voter::Voter,
};

/// Return a Voter from the database via looking up their token ID.
pub async fn voter_by_token(token: &AuthToken<Voter>, voters: &Coll<Voter>) -> Result<Voter> {
    let voter_id = token.id();
    voters
        .find_one(doc! { "_id": *voter_id }, None)
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!("No voter found with ID {:?}", voter_id),
            )
        })
}

/// Return an active Election from the database via ID lookup.
/// An active election is finalised and within its start and end times.
pub async fn active_election_by_id(election_id: Id, elections: &Coll<Election>) -> Result<Election> {
    let now = Utc::now();

    let filter = doc! {
        "_id": *election_id,
        "finalised": true,
        "startTime": { "$lte": now },
        "endTime": { "$gt": now },
    };

    elections
        .find_one(filter, None)
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!("No active election found with ID {:?}", election_id),
            )
        })
}
