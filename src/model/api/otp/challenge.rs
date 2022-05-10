use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{
    errors::Error as JwtError, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use mongodb::bson::doc;
use rocket::{
    http::{Cookie, SameSite, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    time::Duration,
    Request, State,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{model::api::sms::Sms, Config};

use super::code::Code;

pub const CHALLENGE_COOKIE: &str = "challenge";

/// A challenge token tied to a specific SMS number and OTP code.
#[derive(Debug, Serialize, Deserialize)]
pub struct Challenge {
    pub sms: Sms,
    #[serde(rename = "cod")]
    pub code: Code,
}

impl Challenge {
    /// Create a new challenge with a random code.
    pub fn new(sms: Sms) -> Self {
        let code = Code::random();
        println!("{}", code);
        Self { sms, code }
    }

    // Challenge serialization never fails.
    #[allow(clippy::missing_panics_doc)]
    /// Convert into a cookie.
    pub fn into_cookie(self, config: &Config) -> Cookie<'static> {
        let claims = Claims {
            challenge: self,
            expire_at: Utc::now() + config.otp_ttl(),
        };
        Cookie::build(
            CHALLENGE_COOKIE,
            jsonwebtoken::encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret(config.jwt_secret()),
            )
            .unwrap(),
        )
        .max_age(Duration::seconds(config.otp_ttl().num_seconds()))
        .http_only(true)
        .same_site(SameSite::Strict)
        .finish()
    }

    /// Deserialize a challenge from a cookie.
    pub fn from_cookie(cookie: &Cookie<'static>, config: &Config) -> Result<Self, JwtError> {
        jsonwebtoken::decode(
            cookie.value(),
            &DecodingKey::from_secret(config.jwt_secret()),
            &Validation::default(),
        )
        .map(|claims: TokenData<Claims>| claims.claims.challenge)
    }
}

/// Cookie claims: the challenge itself plus an expiry datetime.
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    #[serde(flatten)]
    challenge: Challenge,
    #[serde(rename = "exp", with = "ts_seconds")]
    expire_at: DateTime<Utc>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Challenge {
    type Error = ChallengeError;

    /// Get the challenge from the cookie.
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = req.guard::<&State<Config>>().await.unwrap(); // Valid as `Config` is always managed

        let cookie = try_outcome!(req
            .cookies()
            .get_private(CHALLENGE_COOKIE)
            .into_outcome((Status::Unauthorized, ChallengeError::Missing)));

        let challenge = try_outcome!(Challenge::from_cookie(&cookie, config)
            .map_err(ChallengeError::Jwt)
            .into_outcome(Status::BadRequest));

        request::Outcome::Success(challenge)
    }
}

#[derive(Debug, Error)]
pub enum ChallengeError {
    #[error("Missing `challenge` cookie")]
    Missing,
    #[error(transparent)]
    Jwt(#[from] JwtError),
}
