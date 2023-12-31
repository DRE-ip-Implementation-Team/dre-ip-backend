use argon2::Error as Argon2Error;
use jsonwebtoken::errors::{Error as JwtError, ErrorKind as JwtErrorKind};
use mongodb::{
    bson::oid::Error as OidError,
    error::{Error as DbError, ErrorKind as DbErrorKind},
};
use rocket::{
    http::{Status, StatusClass},
    response::Responder,
};
use std::sync::Arc;
use thiserror::Error;

use crate::{logging::RequestId, model::api::auth::RecaptchaError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Db(DbError),
    #[error(transparent)]
    Oid(#[from] OidError),
    #[error(transparent)]
    Jwt(#[from] JwtError),
    #[error(transparent)]
    Argon2(#[from] Argon2Error),
    #[error(transparent)]
    Recaptcha(#[from] RecaptchaError),
    #[error("{0}: {1}")]
    Status(Status, String),
}

impl From<DbError> for Error {
    fn from(err: DbError) -> Self {
        // Thanks to transactions, we may sometimes get an `Error`
        // wrapped inside a `DbError`.
        if err.get_custom::<Self>().is_some() {
            // Pull the error apart to get the wrapped error by value.
            let DbErrorKind::Custom(arc) = *err.kind else {
                unreachable!()
            };
            let wrapped = arc.downcast::<Self>().unwrap();
            Arc::into_inner(wrapped).expect("multiple refs to DbError")
        } else {
            Self::Db(err)
        }
    }
}

impl Error {
    /// Creates an [`Error::Status`] with [`Status::NotFound`], citing the given cause.
    ///
    /// The cause is a concise sentence-cased description of the resource that was not found.
    ///
    /// Error messages will be displayed as `Not Found: <cause>`.
    pub fn not_found(cause: String) -> Self {
        Self::Status(Status::NotFound, cause)
    }

    /// Get the HTTP response status associated with this error.
    pub fn status(&self) -> Status {
        match self {
            Error::Db(_) => Status::InternalServerError,
            Error::Oid(_) | Error::Argon2(_) => Status::BadRequest,
            Error::Jwt(err) => match err.kind() {
                JwtErrorKind::ExpiredSignature | JwtErrorKind::ImmatureSignature => {
                    Status::Unauthorized
                }
                _ => Status::BadRequest,
            },
            Error::Recaptcha(err) => match err {
                RecaptchaError::ConnectionError(_) => Status::InternalServerError,
                _ => Status::Unauthorized,
            },
            Error::Status(status, _) => *status,
        }
    }
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        let status = self.status();
        let id = req.local_cache(RequestId::next);
        let log_msg = format!("  req{id} {self}");
        if status.class() == StatusClass::ServerError {
            error!("{log_msg}");
        } else {
            warn!("{log_msg}");
        }
        Err(status)
    }
}
