use mongodb::bson::doc;
use rocket::http::Status;

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    mongodb::Coll,
    voter::Voter,
};

/// Return a Voter from the database via looking up their token ID.
pub async fn get_voter_from_token(token: &AuthToken<Voter>, voters: &Coll<Voter>) -> Result<Voter> {
    let voter_id = token.id();
    voters
        .find_one(doc! { "_id": *voter_id }, None)
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!("No voter found with ID {:?}", voter_id),
            )
        })
}
