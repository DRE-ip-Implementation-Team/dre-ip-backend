use mongodb::bson::doc;
use rocket::http::Status;
use rocket::{serde::json::Json, Route};

use crate::{
    error::{Error, Result},
    model::{
        admin::{Admin, AdminCredentials, NewAdmin},
        auth::AuthToken,
        election::{ElectionNoSecrets, ElectionSpec, NewElection},
        mongodb::{Coll, Id},
    },
};

pub fn routes() -> Vec<Route> {
    routes![create_admin, delete_admin, create_election]
}

#[post("/admins", data = "<new_admin>", format = "json")]
async fn create_admin(
    _token: AuthToken<Admin>,
    new_admin: Json<AdminCredentials>,
    admins: Coll<NewAdmin>,
) -> Result<()> {
    // Username uniqueness is enforced by the unique index on the username field.
    let admin: NewAdmin = new_admin.0.into();
    admins.insert_one(admin, None).await?;
    Ok(())
}

#[delete("/admins", data = "<username>", format = "json")]
async fn delete_admin(
    _token: AuthToken<Admin>,
    username: String,
    admins: Coll<Admin>,
) -> Result<()> {
    // Prevent deleting the last admin.
    let count = admins.count_documents(None, None).await?;
    if count == 1 {
        return Err(Error::Status(
            Status::UnprocessableEntity,
            "Cannot delete last admin!".to_string(),
        ));
    }

    let filter = doc! {
        "username": &username,
    };
    let result = admins.delete_one(filter, None).await?;
    if result.deleted_count == 0 {
        Err(Error::not_found(format!("Admin {}", username)))
    } else {
        Ok(())
    }

    // TODO how do we revoke `auth_token`s for this newly-deleted admin?
}

#[post("/elections", data = "<spec>", format = "json")]
async fn create_election(
    _token: AuthToken<Admin>,
    spec: Json<ElectionSpec>,
    new_elections: Coll<NewElection>,
    elections: Coll<ElectionNoSecrets>,
) -> Result<Json<ElectionNoSecrets>> {
    let election: NewElection = spec.0.into();
    let new_id: Id = new_elections
        .insert_one(&election, None)
        .await?
        .inserted_id
        .as_object_id()
        .unwrap() // Valid because the ID comes directly from the DB
        .into();
    let db_election = elections.find_one(new_id.as_doc(), None).await?.unwrap();
    Ok(Json(db_election.erase_secrets()))
}

// TODO election deletion and modification.

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
    async fn create_delete_admin(client: Client, db: Database) {
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

        // Delete the admin.
        let count = admins.count_documents(None, None).await.unwrap();
        assert_eq!(count, 2);
        let response = client
            .delete(uri!(delete_admin))
            .header(ContentType::JSON)
            .body(AdminCredentials::example2().username)
            .dispatch()
            .await;
        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been deleted.
        let count = admins.count_documents(None, None).await.unwrap();
        assert_eq!(count, 1);
        let admin = admins.find_one(None, None).await.unwrap().unwrap();
        assert_eq!(admin.username, AdminCredentials::example().username);
    }

    #[backend_test(admin)]
    async fn create_election(client: Client, db: Database) {
        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(json!(ElectionSpec::current_example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        let elections = Coll::<ElectionMetadata>::from_db(&db);
        let with_name = doc! { "name": &ElectionSpec::current_example().name };
        let inserted_election = elections
            .find_one(with_name.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            ElectionMetadata::from(ElectionSpec::current_example()),
            inserted_election
        );
    }
}
