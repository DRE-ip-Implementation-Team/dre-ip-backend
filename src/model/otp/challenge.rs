use crate::{model::sms::Sms, Config};

use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use mongodb::bson::doc;
use rocket::{
    http::{Cookie, SameSite, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    Request, State,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::code::Code;

#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    sms: Sms,
    #[serde(rename = "cod")]
    code: Code,
}

impl Challenge {
    pub fn sms(self) -> Sms {
        self.sms
    }

    pub fn code(&self) -> Code {
        self.code
    }

    pub fn for_sms(sms: Sms) -> Self {
        let code = Code::default();
        println!("{}", code);
        Self { sms, code }
    }

    pub fn into_cookie(self, config: &Config) -> Cookie<'static> {
        let claims = Claims {
            challenge: self,
            expire_at: Utc::now() + config.otp_ttl(),
        };
        Cookie::build(
            "challenge",
            encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(config.jwt_secret()),
            )
            .unwrap(), // Valid because Challenge serialization never fails
        )
        .max_age(time::Duration::seconds(config.otp_ttl().num_seconds()))
        .same_site(SameSite::Strict)
        .finish()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(flatten)]
    challenge: Challenge,
    #[serde(rename = "exp", with = "ts_seconds")]
    expire_at: DateTime<Utc>,
}

impl Claims {
    pub fn from_str(string: &str, config: &Config) -> Result<Self, JwtError> {
        decode(
            string,
            &DecodingKey::from_secret(config.jwt_secret()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
    }

    pub fn into_challenge(self) -> Challenge {
        self.challenge
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Challenge {
    type Error = ChallengeError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = req.guard::<&State<Config>>().await.unwrap(); // Valid as `Config` is always managed

        let cookie = try_outcome!(req
            .cookies()
            .get_private("challenge")
            .into_outcome((Status::Unauthorized, ChallengeError::Missing)));
        let raw_claims = cookie.value();

        let claims = try_outcome!(Claims::from_str(raw_claims, config)
            .map_err(ChallengeError::Jwt)
            .into_outcome(Status::BadRequest));

        request::Outcome::Success(claims.challenge)
    }
}

#[derive(Debug, Error)]
pub enum ChallengeError {
    #[error("Missing `challenge` cookie")]
    Missing,
    #[error(transparent)]
    Jwt(#[from] JwtError),
}
