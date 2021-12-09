use jsonwebtoken::errors::Error as JwtError;
use mongodb::{
    bson::{doc, oid::ObjectId, DateTime},
    error::Error as DbError,
    Collection,
};
use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request, State,
};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::conf;

use self::claims::Claims;

use super::sms::Sms;

pub mod claims;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expire_at: Option<DateTime>,
    pub sms: Sms,
}

impl User {
    /// Returns a [`User`] with an `sms` that expires a number of seconds from the current system time.
    ///
    /// See [`Config`] to customise the number of seconds until expiry.
    pub fn new(sms: Sms) -> Self {
        Self {
            id: None,
            expire_at: Some(DateTime::from_system_time(
                SystemTime::now() + Duration::from_secs(conf!(otp_ttl)),
            )),
            sms,
        }
    }

    pub fn expire_at(&self) -> Option<DateTime> {
        self.expire_at
    }
}

pub type Users = Collection<User>;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = UserAuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let users: &State<Users> = req.guard().await.unwrap();
        if let Some(cookie) = req.cookies().get("auth_token") {
            let token = cookie.value();
            match token.parse::<Claims>() {
                Ok(claims) => {
                    let user_id = claims.user_id.unwrap();
                    match users.find_one(doc! { "_id": user_id }, None).await {
                        Ok(result) => match result {
                            Some(user) => Outcome::Success(user),
                            None => {
                                // No user found
                                Outcome::Failure((
                                    Status::InternalServerError,
                                    UserAuthError::NoUser,
                                ))
                            }
                        },
                        Err(db_err) => {
                            // DB failed to fetch user
                            Outcome::Failure((
                                Status::InternalServerError,
                                UserAuthError::DbError(db_err),
                            ))
                        }
                    }
                }
                Err(jwt_err) => {
                    return Outcome::Failure((Status::BadRequest, UserAuthError::JwtError(jwt_err)))
                }
            }
        } else {
            // No `user_id` cookie set
            Outcome::Failure((Status::Unauthorized, UserAuthError::NoCookie))
        }
    }
}

#[derive(Debug)]
pub enum UserAuthError {
    NoCookie,
    BadCookie(String),
    NoUser,
    DbError(DbError),
    JwtError(JwtError),
}
