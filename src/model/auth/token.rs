use std::marker::PhantomData;

use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{
    errors::Error as JwtError, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use rocket::{
    http::{Cookie, SameSite},
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    Request, State,
};
use serde::{Deserialize, Serialize};
use time;

use crate::model::mongodb::Id;
use crate::Config;

use super::user::{Rights, User};

pub const AUTH_TOKEN_COOKIE: &str = "auth_token";

/// An authentication token representing a specific user with specific rights.
#[derive(Serialize, Deserialize)]
pub struct AuthToken<U> {
    pub id: Id,
    #[serde(rename = "rgt")]
    pub rights: Rights,
    #[serde(skip)]
    phantom: PhantomData<U>,
}

impl<U> AuthToken<U> {
    /// Does this token permit the given rights?
    pub fn permits(&self, target: Rights) -> bool {
        self.rights == target
    }
}

impl<U> AuthToken<U>
where
    U: User,
{
    /// Create a new [`AuthToken`] for the given user, with the correct rights for that user type.
    pub fn new(user: &U) -> Self {
        Self {
            id: user.id(),
            rights: U::RIGHTS,
            phantom: PhantomData,
        }
    }

    // JWT encoding is infallible with default settings
    #[allow(clippy::missing_panics_doc)]
    /// Serialize this cookie into a token.
    pub fn into_cookie(self, config: &Config) -> Cookie<'static> {
        let claims = Claims {
            token: self,
            expire_at: Utc::now() + config.auth_ttl(),
        };

        let token = jsonwebtoken::encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret()),
        )
        .unwrap();

        Cookie::build(AUTH_TOKEN_COOKIE, token)
            .max_age(time::Duration::seconds(config.auth_ttl().num_seconds()))
            .same_site(SameSite::Strict)
            .finish()
    }

    /// Deserialize a token from a cookie.
    pub fn from_cookie(cookie: &Cookie<'static>, config: &Config) -> Result<Self, JwtError> {
        jsonwebtoken::decode(
            cookie.value(),
            &DecodingKey::from_secret(config.jwt_secret()),
            &Validation::default(),
        )
        .map(|claims: TokenData<Claims<U>>| claims.claims.token)
    }
}

/// Cookie claims: the token itself plus an expiry datetime.
#[derive(Serialize, Deserialize)]
struct Claims<U> {
    #[serde(flatten, bound = "")]
    token: AuthToken<U>,
    #[serde(rename = "exp", with = "ts_seconds")]
    expire_at: DateTime<Utc>,
}

#[rocket::async_trait]
impl<'r, U> FromRequest<'r> for AuthToken<U>
where
    U: User,
{
    type Error = JwtError;

    /// Get an [`AuthToken`] from the cookie and verify that it has the correct rights for this user
    /// type.
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        // Unwrap is valid as `Config` is always managed
        let config = req.guard::<&State<Config>>().await.unwrap();

        // Forward to any routes that do not require an authentication token
        let cookie = try_outcome!(req.cookies().get(AUTH_TOKEN_COOKIE).or_forward(()));

        let token: Self =
            try_outcome!(Self::from_cookie(cookie, config).or_forward(()));

        if token.permits(U::RIGHTS) {
            request::Outcome::Success(token)
        } else {
            request::Outcome::Forward(())
        }
    }
}
