use mongodb::bson::doc;
use rocket::{http::Status, serde::json::Json, Route};

use crate::{
    error::{Error, Result},
    model::{
        auth::token::AuthToken,
        election::Group,
        mongodb::{bson::Id, collection::Coll},
        voter::{db::DbVoter, Voter},
    },
};

pub fn routes() -> Vec<Route> {
    routes![get_voters_groups]
}

#[get("/voter/elections/<election_id>/groups")]
async fn get_voters_groups(
    token: AuthToken<Voter>,
    voters: Coll<DbVoter>,
    election_id: Id,
) -> Result<Json<Vec<Group>>> {
    let voter_id = token.id();
    let voter = voters
        .find_one(doc! { "_id": *voter_id }, None)
        .await?
        .ok_or_else(|| {
            Error::Status(
                Status::NotFound,
                format!("No voter found with ID {:?}", voter_id),
            )
        })?;
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
