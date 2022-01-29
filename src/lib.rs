#[macro_use]
extern crate rocket;

#[cfg(test)]
#[macro_use]
extern crate db_test;

use chrono::Duration;
use mongodb::{Client, Database};
use rocket::{fairing::AdHoc, Build, Rocket};
use serde::Deserialize;

pub mod api;
pub mod error;
pub mod model;

#[cfg(not(test))]
static DATABASE: &'static str = "dreip";

#[cfg(test)]
static DATABASE: &'static str = "test";

pub async fn build() -> Rocket<Build> {
    let (rocket, _) = rocket_with_db().await;
    rocket
}

pub(crate) async fn rocket_with_db() -> (Rocket<Build>, Database) {
    let rocket = rocket::build();
    let figment = rocket.figment();

    let db_uri = figment
        .extract_inner::<String>("db_uri")
        .expect("`db_uri` not set");
    let client = Client::with_uri_str(&db_uri).await.expect(&format!(
        "Could not connect to database with `db_uri` \"{}\"",
        db_uri
    ));
    let db = client.database(DATABASE);

    (
        rocket
            .mount("/", api::routes())
            .attach(AdHoc::config::<Config>())
            .manage(client)
            .manage(db.clone()),
        db,
    )
}

#[derive(Deserialize)]
pub struct Config {
    otp_ttl: u64,
    jwt_secret: String,
    auth_ttl: u64,
}

impl Config {
    /// Seconds until the OTP challenge expires.
    /// Configured via `OTP_TTL`.
    pub fn otp_ttl(&self) -> Duration {
        Duration::seconds(self.otp_ttl as i64)
    }

    /// Key used to encrypt JWTs
    /// Configured via `JWT_SECRET`.
    pub fn jwt_secret(&self) -> &[u8] {
        self.jwt_secret.as_bytes()
    }

    /// Seconds until the authentication token expires
    /// Configured via `AUTH_TTL`.
    pub fn auth_ttl(&self) -> Duration {
        Duration::seconds(self.auth_ttl as i64)
    }
}

#[cfg(test)]
async fn client_and_db() -> (rocket::local::asynchronous::Client, Database) {
    let (rocket, db) = rocket_with_db().await;
    let client = rocket::local::asynchronous::Client::tracked(rocket)
        .await
        .unwrap();
    (client, db)
}
