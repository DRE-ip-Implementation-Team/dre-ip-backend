use std::marker::PhantomData;

use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{
    errors::Error as JwtError, DecodingKey, EncodingKey, Header, TokenData, Validation,
};
use rocket::{
    http::{Cookie, SameSite, Status},
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
    id: Id,
    #[serde(rename = "rgt")]
    rights: Rights,
    #[serde(skip)]
    phantom: PhantomData<U>,
}

impl<U> AuthToken<U> {
    /// Get the user ID.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Get the user's rights.
    pub fn rights(&self) -> Rights {
        self.rights
    }

    /// Does this token permit the given rights?
    pub fn permits(&self, target: Rights) -> bool {
        self.rights == target
    }
}

impl<U> AuthToken<U>
where
    U: User,
{
    /// Create a new [`AuthToken`] for the given user, with the correct rights for
    /// that user type.
    pub fn new(user: &U) -> Self {
        Self {
            id: user.id(),
            rights: U::RIGHTS,
            phantom: PhantomData,
        }
    }

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
        .unwrap(); // Infallible.

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

    /// Get an AuthToken from the cookie and verify that it has the correct rights
    /// for this user type.
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = req.guard::<&State<Config>>().await.unwrap(); // Valid as `Config` is always managed

        let cookie = try_outcome!(req.cookies().get(AUTH_TOKEN_COOKIE).or_forward(()));
        let token: Self =
            try_outcome!(Self::from_cookie(cookie, config).into_outcome(Status::Unauthorized));

        if token.permits(U::RIGHTS) {
            request::Outcome::Success(token)
        } else {
            request::Outcome::Forward(())
        }
    }
}
