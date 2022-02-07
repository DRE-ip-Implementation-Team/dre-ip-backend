#[macro_use]
extern crate rocket;

#[cfg(test)]
#[macro_use]
extern crate backend_test;

use chrono::Duration;
use mongodb::Client;
use rocket::{fairing::AdHoc, Build, Rocket};
use serde::Deserialize;

pub mod api;
pub mod error;
pub mod model;

pub async fn build() -> Rocket<Build> {
    rocket_for_db_client(db_client().await).await
}

pub(crate) async fn db_client() -> Client {
    let db_uri = env!("db_uri");
    Client::with_uri_str(db_uri)
        .await
        .unwrap_or_else(|err| panic!("{}", err))
}

#[cfg(not(test))]
const DATABASE: &str = "dreip";

#[cfg(test)]
const DATABASE: &str = "test";

pub(crate) async fn rocket_for_db_client(client: Client) -> Rocket<Build> {
    let db = client.database(DATABASE);

    rocket::build()
        .mount("/", api::routes())
        .attach(AdHoc::config::<Config>())
        .manage(client)
        .manage(db)
}

#[derive(Deserialize)]
pub struct Config {
    otp_ttl: u32,
    jwt_secret: String,
    auth_ttl: u32,
}

impl Config {
    /// Seconds until the OTP challenge expires.
    /// Configured via `OTP_TTL`.
    pub fn otp_ttl(&self) -> Duration {
        Duration::seconds(self.otp_ttl.into())
    }

    /// Key used to encrypt JWTs
    /// Configured via `JWT_SECRET`.
    pub fn jwt_secret(&self) -> &[u8] {
        self.jwt_secret.as_bytes()
    }

    /// Seconds until the authentication token expires
    /// Configured via `AUTH_TTL`.
    pub fn auth_ttl(&self) -> Duration {
        Duration::seconds(self.auth_ttl.into())
    }
}
