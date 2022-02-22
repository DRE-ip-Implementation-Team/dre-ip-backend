use std::ops::{Deref, DerefMut};

use mongodb::bson::doc;
use rocket::{
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest, Request},
};
use serde::{Deserialize, Serialize};

use crate::{
    error::Error,
    model::mongodb::{Coll, Id},
};

use super::election_core::ElectionCore;

/// An election from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
pub struct Election {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub election: ElectionCore,
}

impl Deref for Election {
    type Target = ElectionCore;

    fn deref(&self) -> &Self::Target {
        &self.election
    }
}

impl DerefMut for Election {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.election
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Election {
    type Error = Error;

    /// XXX: This impl is predicated on the [`Id`] of the [`Election`] being the 3rd non-empty
    /// segment of the request URI.
    /// As such, it is fragile to changes in the position of the [`Id`] in the URI.
    /// However, it improves the ergonomics of most election endpoints and the current URIs are
    /// unlikely to change in a breaking manner.
    /// Such changes could not occur without failing tests involving affected endpoints.
    /// If needed, it is trivial to revert to fetching the election from the DB manually.
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Assume election ID is in the 3rd segment
        let election_id = req.param::<Id>(2).unwrap().unwrap();

        // See `Coll::from_request`
        let elections = req.guard::<Coll<Election>>().await.unwrap();

        // Query for election
        let maybe_election = try_outcome!(elections
            .find_one(election_id.as_doc(), None)
            .await
            .map_err(Error::Db)
            .into_outcome(Status::NotFound));
        let election = try_outcome!(maybe_election
            .ok_or_else(|| Error::not_found(format!("Election with ID '{}'", election_id)))
            .into_outcome(Status::NotFound));

        request::Outcome::Success(election)
    }
}
