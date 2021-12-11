use crate::{conf, model::sms::Sms};

use chrono::{Duration, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use mongodb::bson::doc;
use rocket::{
    http::{Cookie, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    Request,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use std::str::FromStr;

use super::code::Code;

#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    sms: Sms,
    #[serde(rename = "cod")]
    code: Code,
    #[serde(rename = "exp")]
    expire_at: u64,
}

impl Challenge {
    pub fn sms(self) -> Sms {
        self.sms
    }

    pub fn code(&self) -> Code {
        self.code
    }

    pub fn cookie(sms: Sms) -> Cookie<'static> {
        let challenge = Self {
            sms,
            code: dbg!(Code::default()),
            expire_at: (Utc::now() + Duration::seconds(conf!(otp_ttl) as i64)).timestamp() as u64,
        };
        Cookie::build(
            "challenge",
            encode(
                &Header::default(),
                &challenge,
                &EncodingKey::from_secret(conf!(jwt_secret)),
            )
            .unwrap(), // Valid because Challenge serialization never fails
        )
        .max_age(time::Duration::seconds(conf!(otp_ttl) as i64))
        .finish()
    }
}

impl FromStr for Challenge {
    type Err = JwtError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        decode(
            string,
            &DecodingKey::from_secret(conf!(jwt_secret)),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Challenge {
    type Error = ChallengeError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookie = try_outcome!(req
            .cookies()
            .get_private("challenge")
            .into_outcome((Status::Unauthorized, ChallengeError::Missing)));
        let token = cookie.value();
        request::Outcome::Success(try_outcome!(token
            .parse::<Challenge>()
            .map_err(ChallengeError::Jwt)
            .into_outcome(Status::BadRequest)))
    }
}

#[derive(Debug, Error)]
pub enum ChallengeError {
    #[error("Missing `challenge` cookie")]
    Missing,
    #[error(transparent)]
    Jwt(#[from] JwtError),
}
