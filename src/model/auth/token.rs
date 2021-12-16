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
pub enum Rights {
    Voter = 0,
    Admin = 1,
}

impl Display for Rights {
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
    fn rights() -> Rights;
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
            .into_outcome(Status::Unauthorized));
        if claims.permits(U::rights()) {
            request::Outcome::Success(Token(PhantomData))
        } else if let Rights::Voter = U::rights() {
            request::Outcome::Failure((
                Status::Forbidden,
                TokenError::NotPermitted {
                    target: U::rights(),
                    actual: claims.rights(),
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
