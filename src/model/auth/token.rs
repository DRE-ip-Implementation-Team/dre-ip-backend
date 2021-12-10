use std::{fmt::Display, marker::PhantomData};

use jsonwebtoken::errors::Error as JwtError;
use rocket::{
    http::Status,
    outcome::{try_outcome, IntoOutcome},
    request::{self, FromRequest},
    Request,
};
use serde_repr::{Deserialize_repr, Serialize_repr};
use thiserror::Error;

use super::claims::Claims;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum Privileges {
    Voter = 0,
    Admin = 1,
}

impl Display for Privileges {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}",
            match self {
                Self::Voter => "voter",
                Self::Admin => "admin",
            }
        )
    }
}

pub trait User {
    fn privileges() -> Privileges;
}

pub struct Token<U: User>(PhantomData<U>);

#[rocket::async_trait]
impl<'r, U> FromRequest<'r> for Token<U>
where
    U: User,
{
    type Error = TokenError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let cookie = try_outcome!(req
            .cookies()
            .get("auth_token")
            .into_outcome((Status::Unauthorized, TokenError::Missing)));
        let token = cookie.value();
        let claims = try_outcome!(token
            .parse::<Claims>()
            .map_err(TokenError::Jwt)
            .into_outcome(Status::BadRequest));
        if claims.permits(U::privileges()) {
            request::Outcome::Success(Token(PhantomData))
        } else {
            if let Privileges::Voter = U::privileges() {
                request::Outcome::Failure((
                    Status::Unauthorized,
                    TokenError::NotPermitted {
                        target: U::privileges(),
                        actual: claims.privileges(),
                    },
                ))
            } else {
                request::Outcome::Forward(())
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum TokenError {
    #[error("Missing `auth_token` cookie")]
    Missing,
    #[error("Required {target} privileges, got {actual}")]
    NotPermitted {
        target: Privileges,
        actual: Privileges,
    },
    #[error(transparent)]
    Jwt(#[from] JwtError),
}
