use mongodb::bson::doc;
use rocket::{http::Status, serde::json::Json, Route};

use crate::error::{Error, Result};
use crate::model::{
    auth::AuthToken,
    mongodb::{Coll, Id},
    voter::Voter,
};

use super::common::voter_by_token;

pub fn routes() -> Vec<Route> {
    routes![get_voters_groups]
}

#[get("/voter/elections/<election_id>/groups")]
async fn get_voters_groups(
    token: AuthToken<Voter>,
    voters: Coll<Voter>,
    election_id: Id,
) -> Result<Json<Vec<Id>>> {
    let voter = voter_by_token(&token, &voters).await?;
    let groups = voter
        .election_groups()
        .get(&election_id)
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!(
                    "Voter does not participate in election with ID {}",
                    *election_id
                ),
            )
        })?
        .clone();
    Ok(Json(groups))
}

// #[post("/voter/elections/<election_id>/groups/join", data = "<groups>")]
// async fn join_groups(
//     token: Token<Voter>,
//     election_id: Id,
//     voters: Coll<Voter>,
//     elections: Coll<Election>,
//     groups: Json<Vec<String>>,
// ) -> Result<()> {
//     let valid_groups = elections
//         .find_one(
//             doc! {
//                 "_id": *election_id,
//             },
//             None,
//         )
//         .await?
//         .ok_or_else(|| {
//             Error::Status(
//                 Status::NotFound,
//                 format!("No election found with ID {}", *election_id),
//             )
//         })?
//         .groups();
//     let submitted_groups = groups.groups.into_iter().collect::<HashSet<_>>();
//     if !valid_groups.is_superset(&submitted_groups) {}

//     let voter_id = token.id();
//     voters.update_one(
//         doc! {
//             "_id": voter_id,
//         },
//         doc! {
//             "election_groups": {
//                 "$set": {
//                     election_id.to_string(): submitted_groups,
//                 }
//             }
//         },
//         None,
//     );
//     Ok(())
// }
