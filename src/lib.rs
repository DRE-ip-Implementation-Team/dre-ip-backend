#[macro_use]
extern crate rocket;

#[cfg(test)]
#[macro_use]
extern crate backend_test;

use aws_sdk_sns::Client as SnsClient;
use chrono::Duration;
use mongodb::Client;
use rocket::{fairing::AdHoc, Build, Rocket};
use serde::Deserialize;

pub mod api;
pub mod error;
pub mod model;

pub async fn build() -> Rocket<Build> {
    rocket_for_db_and_notifier(db_client().await, &database(), notifier().await)
}

pub(crate) async fn db_client() -> Client {
    Client::with_uri_str(env!("db_uri")).await.unwrap()
}

pub(crate) async fn notifier() -> SnsClient {
    SnsClient::new(&aws_config::load_from_env().await)
}

/// Get the name of the database to use.
/// This is randomised for tests so different tests do not collide.
fn database() -> String {
    #[cfg(not(test))]
    return "dreip".to_string();

    #[cfg(test)]
    {
        let random: u32 = rand::random();
        let db = format!("test{}", random);
        println!("Using database {}", db);
        db
    }
}

/// Used in both the application entry point and the `backend_test` macro
pub(crate) fn rocket_for_db_and_notifier(
    client: Client,
    db: &str,
    notifier: SnsClient,
) -> Rocket<Build> {
    let db = client.database(db);

    rocket::build()
        .mount("/", api::routes())
        .attach(AdHoc::config::<Config>())
        .manage(client)
        .manage(db)
        .manage(notifier)
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
