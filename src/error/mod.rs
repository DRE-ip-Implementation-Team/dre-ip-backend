use argon2::Error as Argon2Error;
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use mongodb::error::Error as DbError;
use rocket::{http::Status, response::Responder};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Db(#[from] DbError),
    #[error(transparent)]
    Jwt(#[from] JwtError),
    #[error(transparent)]
    Argon2(#[from] Argon2Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        println!("{:?}", self);
        Err(match self {
            Self::BadRequest(_) => Status::BadRequest,
            Self::NotFound(_) => Status::NotFound,
            Self::Unauthorized(_) => Status::Unauthorized,
            Self::Db(_) => Status::InternalServerError,
            Self::Jwt(err) => match err.into_kind() {
                JwtErrorKind::ExpiredSignature | JwtErrorKind::ImmatureSignature => {
                    Status::Unauthorized
                }
                _ => Status::BadRequest,
            },
            Self::Argon2(_) => Status::BadRequest,
        })
    }
}
