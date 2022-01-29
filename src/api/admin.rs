use mongodb::bson::doc;
use rocket::{serde::json::Json, Route};

use crate::{
    error::Result,
    model::{
        admin::{Admin, Credentials},
        auth::token::AuthToken,
        election::{db::DbElection, Election, ElectionSpec},
        mongodb::collection::Coll,
    },
};

pub fn routes() -> Vec<Route> {
    routes![create_admin, create_election]
}

#[post("/admins", data = "<credentials>", format = "json")]
async fn create_admin(
    _token: AuthToken<Admin>,
    credentials: Json<Credentials<'_>>,
    admins: Coll<Admin>,
) -> Result<()> {
    let admin = credentials.into_admin();
    admins.insert_one(admin, None).await?;
    Ok(())
}

#[post("/elections", data = "<spec>", format = "json")]
async fn create_election(
    spec: Json<ElectionSpec>,
    elections: Coll<Election>,
) -> Result<Json<DbElection>> {
    let election = spec.0.into();
    let id = elections
        .insert_one(&election, None)
        .await?
        .inserted_id
        .as_object_id()
        .unwrap(); // Valid because the ID comes directly from the DB
    let db_election = DbElection::new(id.into(), election);
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

    use crate::{api::auth::login_as_admin, model::election::ElectionSpec};

    use super::*;

    #[backend_test]
    async fn create_admin(client: Client, db: Database) {
        login_as_admin(&client, &db).await;

        // Create admin
        let response = client
            .post(uri!(create_admin))
            .header(ContentType::JSON)
            .body(json!(Credentials::example2()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        // Ensure the admin has been inserted
        let admins = Coll::<Admin>::from_db(&db);
        let with_username = doc! { "username": Admin::example2().username() };
        let inserted_admin = admins
            .find_one(with_username.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(Admin::example2().username(), inserted_admin.username());
    }

    #[backend_test]
    async fn create_election(client: Client, db: Database) {
        login_as_admin(&client, &db).await;

        let response = client
            .post(uri!(create_election))
            .header(ContentType::JSON)
            .body(json!(ElectionSpec::example()).to_string())
            .dispatch()
            .await;

        assert_eq!(Status::Ok, response.status());

        let elections = Coll::<ElectionSpec>::from_db(&db);
        let with_name = doc! { "name": ElectionSpec::example().name() };
        let inserted_election = elections
            .find_one(with_name.clone(), None)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(ElectionSpec::example(), inserted_election);
    }
}
