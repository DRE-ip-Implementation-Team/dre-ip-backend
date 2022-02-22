use chrono::Utc;
use mongodb::bson::doc;

use crate::error::{Error, Result};
use crate::model::{
    election::Election,
    mongodb::{Coll, Id},
};

/// Return an active Election from the database via ID lookup.
/// An active election is finalised and within its start and end times.
pub async fn active_election_by_id(
    election_id: Id,
    elections: &Coll<Election>,
) -> Result<Election> {
    let now = Utc::now();

    let is_active = doc! {
        "_id": *election_id,
        "finalised": true,
        "startTime": { "$lte": now },
        "endTime": { "$gt": now },
    };

    elections
        .find_one(is_active, None)
        .await?
        .ok_or_else(|| Error::not_found(format!("Active election with ID '{}'", election_id)))
}
