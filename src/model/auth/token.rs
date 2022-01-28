use std::marker::PhantomData;

use chrono::{serde::ts_seconds, DateTime, Utc};
use jsonwebtoken::{
    decode, encode, errors::Error as JwtError, Algorithm, DecodingKey, EncodingKey, Header,
    Validation,
};
use rocket::{
    http::{Cookie, SameSite, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    Request, State,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use time;

use crate::{
    model::mongodb::{bson::Id, entity::DbEntity},
    Config,
};

use super::user::{Rights, User};

#[derive(Serialize, Deserialize)]
pub struct AuthToken<U> {
    id: Id,
    #[serde(rename = "rgt")]
    rights: Rights,
    #[serde(skip)]
    phantom: PhantomData<U>,
}

impl<U> AuthToken<U> {
    pub fn id(&self) -> Id {
        self.id
    }

    pub fn rights(&self) -> Rights {
        self.rights
    }

    pub fn permits(&self, target: Rights) -> bool {
        self.rights == target
    }
}

impl<U> AuthToken<U>
where
    U: User,
{
    pub fn for_user(voter: &U::DbUser) -> Self {
        Self {
            id: voter.id(),
            rights: U::rights(),
            phantom: PhantomData::<U>,
        }
    }

    pub fn into_cookie(self, config: &Config) -> Cookie<'static> {
        let claims = Claims {
            token: self,
            expire_at: Utc::now() + config.auth_ttl(),
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret()),
        )
        .unwrap(); // Valid because Claims serialization never fails
        Cookie::build("auth_token", token)
            .max_age(time::Duration::seconds(config.auth_ttl().num_seconds()))
            .same_site(SameSite::Strict)
            .finish()
    }
}

// #[rocket::async_trait]
// impl<'r, U> Responder<'r, 'static> for AuthToken<U> {
//     fn respond_to(self, req: &'r Request<'_>) -> response::Result<'static> {
//         let config = req
//             .guard::<&State<Config>>()
//             .await
//             .expect("`Config` is not managed");
//         let cookies = req.guard::<CookieJar>().await.unwrap(); // Valid as `CookieJar` is always available
//         let response = Response::default();
//         response.cookie()
//     }
// }

#[derive(Serialize, Deserialize)]
pub struct Claims<U> {
    #[serde(flatten, bound = "")]
    token: AuthToken<U>,
    #[serde(rename = "exp", with = "ts_seconds")]
    expire_at: DateTime<Utc>,
}

impl<U> Claims<U> {
    pub fn from_str(string: &str, config: &Config) -> Result<Self, JwtError> {
        decode(
            string,
            &DecodingKey::from_secret(config.jwt_secret()),
            &Validation::new(Algorithm::HS256),
        )
        .map(|data| data.claims)
    }

    pub fn into_token(self) -> AuthToken<U> {
        self.token
    }
}
#[rocket::async_trait]
impl<'r, U> FromRequest<'r> for AuthToken<U>
where
    U: User,
{
    type Error = TokenError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let config = req.guard::<&State<Config>>().await.unwrap(); // Valid as `Config` is always managed

        let cookie = try_outcome!(req.cookies().get("auth_token").or_forward(()));
        let raw_claims = cookie.value();

        let claims = try_outcome!(Claims::from_str(raw_claims, config)
            .map_err(TokenError::Jwt)
            .into_outcome(Status::Unauthorized));

        let token = claims.token;

        if token.permits(U::rights()) {
            request::Outcome::Success(token)
        } else if let Rights::Voter = U::rights() {
            request::Outcome::Failure((
                Status::Forbidden,
                TokenError::NotPermitted {
                    target: U::rights(),
                    actual: token.rights,
                },
            ))
        } else {
            request::Outcome::Forward(())
        }
    }
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Missing `auth_token` cookie")]
    Missing,
    #[error("Required {target} rights, got {actual} rights")]
    NotPermitted { target: Rights, actual: Rights },
    #[error(transparent)]
    Jwt(#[from] JwtError),
}
