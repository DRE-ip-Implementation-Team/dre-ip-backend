use jsonwebtoken::errors::Error as JwtError;
use mongodb::{
    bson::{doc, oid::ObjectId},
    error::Error as DbError,
    Collection,
};
use rocket::{
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};

use self::claims::Claims;

use super::sms::Sms;

pub mod claims;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    sms: Sms,
}

impl User {
    /// Returns a [`User`] with an `sms` that expires a number of seconds from the current system time.
    ///
    /// See [`Config`] to customise the number of seconds until expiry.
    pub fn new(sms: Sms) -> Self {
        Self { id: None, sms }
    }
}

pub type Users = Collection<User>;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = AuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let token = try_outcome!(req
            .cookies()
            .get("auth_token")
            .into_outcome((Status::Unauthorized, AuthError::NoCookie)))
        .value();
        let claims = try_outcome!(token
            .parse::<Claims>()
            .map_err(AuthError::JwtError)
            .into_outcome(Status::BadRequest));
        let user_id = claims.user_id.unwrap();
        let users: &State<Users> = req.guard().await.unwrap();
        let result = try_outcome!(users
            .find_one(doc! { "_id": user_id }, None)
            .await
            .map_err(AuthError::DbError)
            .into_outcome(Status::InternalServerError));
        let user = try_outcome!(result
            .ok_or(AuthError::NoUser)
            .into_outcome(Status::InternalServerError));
        Outcome::Success(user)
    }
}

#[derive(Debug)]
pub enum AuthError {
    NoCookie,
    NoUser,
    DbError(DbError),
    JwtError(JwtError),
}
