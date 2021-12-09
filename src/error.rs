use rocket::{http::Status, response::Responder};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error(transparent)]
    Db(#[from] mongodb::error::Error),
    #[error(transparent)]
    OidParse(#[from] mongodb::bson::oid::Error),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, _: &'r rocket::Request<'_>) -> rocket::response::Result<'o> {
        Err(match self {
            Self::Jwt(_) | Self::Db(_) => Status::InternalServerError,
            Self::OidParse(_) | Self::BadRequest(_) => Status::BadRequest,
            Self::Unauthorized(_) => Status::BadRequest,
            Self::NotFound(_) => Status::NotFound,
        })
    }
}
