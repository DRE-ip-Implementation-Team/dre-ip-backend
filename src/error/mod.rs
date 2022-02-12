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
    #[error("{0}: {1}")]
    Status(Status, String),
}

impl From<Error> for Status {
    fn from(error: Error) -> Self {
        match error {
            Error::Argon2(_) => Status::BadRequest,
            Error::Db(_) => Status::InternalServerError,
            Error::Jwt(err) => match err.into_kind() {
                JwtErrorKind::ExpiredSignature | JwtErrorKind::ImmatureSignature => {
                    Status::Unauthorized
                }
                _ => Status::BadRequest,
            },
            Error::Status(status, _) => status,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        println!("{:?}", self);
        Err(self.into())
    }
}
