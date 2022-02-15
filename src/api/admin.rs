use mongodb::bson::doc;
use rocket::{serde::json::Json, Route};

use crate::{
    error::Result,
    model::{
        admin::{Admin, AdminCredentials, NewAdmin},
        auth::AuthToken,
        election::{Election, ElectionSpec, NewElection},
        mongodb::Coll,
    },
};

pub fn routes() -> Vec<Route> {
    routes![create_admin, create_election]
}

#[post("/admins", data = "<new_admin>", format = "json")]
async fn create_admin(
    _token: AuthToken<Admin>,
    new_admin: Json<AdminCredentials>,
    admins: Coll<NewAdmin>,
) -> Result<()> {
    let admin: NewAdmin = new_admin.0.into();
    admins.insert_one(admin, None).await?;
    Ok(())
}

#[post("/elections", data = "<spec>", format = "json")]
async fn create_election(
    spec: Json<ElectionSpec>,
    new_elections: Coll<NewElection>,
    elections: Coll<Election>,
) -> Result<Json<Election>> {
    let election: NewElection = spec.0.into();
    let new_id = new_elections
        .insert_one(&election, None)
        .await?
        .inserted_id
        .as_object_id()
        .unwrap(); // Valid because the ID comes directly from the DB
    let db_election = elections
        .find_one(doc! { "_id": new_id }, None)
        .await?
        .unwrap();
    Ok(Json(db_election))
}

#[cfg(test)]
mod tests {
    use mongodb::Database;
    use rocket::{
        http::{ContentType, Status},
        local::asynchronous::Client,
        serde::json::serde_json::json,
    };

    use crate::model::election::ElectionMetadata;

    use super::*;

    #[backend_test(admin)]
    async fn create_admin(client: Client, db: Database) {
        // Create admin
        let response = client
            .post(uri!(create_admin))
            .header(ContentType::JSON)
            .body(json!(AdminCredentials::example2()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been inserted
        let admins = Coll::<Admin>::from_db(&db);
        let with_username = doc! { "username": &NewAdmin::example2().username };
        let inserted_admin = admins
            .find_one(with_username.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(NewAdmin::example2().username, inserted_admin.username);
    }

    #[backend_test(admin)]
    async fn create_election(client: Client, db: Database) {
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(json!(ElectionSpec::finalised_example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        let elections = Coll::<ElectionMetadata>::from_db(&db);
        let with_name = doc! { "name": &ElectionSpec::finalised_example().metadata.name };
        let inserted_election = elections
            .find_one(with_name.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            ElectionMetadata::from(ElectionSpec::finalised_example()),
            inserted_election
        );
    }
}
