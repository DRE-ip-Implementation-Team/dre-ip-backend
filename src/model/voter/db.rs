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
    model::{
        auth::AuthToken,
        mongodb::{Coll, Id},
    },
};

use super::voter_core::VoterCore;

/// A voter user from the database, with its unique ID.
#[derive(Serialize, Deserialize)]
pub struct Voter {
    #[serde(rename = "_id")]
    pub id: Id,
    #[serde(flatten)]
    pub voter: VoterCore,
}

impl Deref for Voter {
    type Target = VoterCore;

    fn deref(&self) -> &Self::Target {
        &self.voter
    }
}

impl DerefMut for Voter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.voter
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Voter {
    type Error = Error;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Check for voter authentication
        let auth_token = try_outcome!(req
            .guard::<AuthToken<Self>>()
            .await
            .map_failure(|(status, jwt_error)| (status, Error::Jwt(jwt_error))));

        // See `Coll::from_request`
        let voters = req.guard::<Coll<Self>>().await.unwrap();

        // Query for auth'ed voter
        let maybe_voter = try_outcome!(voters
            .find_one(auth_token.id.as_doc(), None)
            .await
            .map_err(Error::Db)
            .into_outcome(Status::NotFound));
        let voter = try_outcome!(maybe_voter
            .ok_or_else(|| Error::not_found(format!("Voter with ID '{}'", auth_token.id)))
            .into_outcome(Status::NotFound));

        request::Outcome::Success(voter)
    }
}
