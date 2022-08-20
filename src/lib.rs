#[macro_use]
extern crate rocket;

#[cfg(test)]
#[macro_use]
extern crate backend_test;

use std::sync::Arc;

use aws_sdk_sns::Client as SnsClient;
use chrono::Duration;
use mongodb::Client;
use rocket::{
    fairing::AdHoc,
    shield::{NoSniff, Shield},
    tokio::sync::Mutex,
    Build, Rocket,
};
use serde::Deserialize;

pub mod api;
pub mod error;
pub mod model;
pub mod scheduled_task;

use crate::model::{
    db::{admin::ensure_admin_exists, election::ElectionFinalizers as RawElectionFinalizers},
    mongodb::{ensure_election_id_counter_exists, Coll},
};

pub async fn build() -> Rocket<Build> {
    rocket_for_db_and_notifier(db_client().await, &database(), notifier().await).await
}

pub(crate) async fn db_client() -> Client {
    let db_uri = std::env::var("db_uri").expect("db_uri envvar wasn't present");
    Client::with_uri_str(db_uri).await.unwrap()
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
pub(crate) async fn rocket_for_db_and_notifier(
    client: Client,
    db: &str,
    notifier: SnsClient,
) -> Rocket<Build> {
    // Create the database reference.
    let db = client.database(db);

    // Create an election finalizer for every election that needs one.
    let mut election_finalizers = RawElectionFinalizers::new();
    election_finalizers
        .schedule_elections(&db)
        .await
        .expect("Failed to contact database during election finalizer init");

    // Ensure there is at least one admin user.
    ensure_admin_exists(&Coll::from_db(&db))
        .await
        .expect("Failed to contact database during admin user init");

    // Ensure the global election ID counter exists.
    ensure_election_id_counter_exists(&Coll::from_db(&db))
        .await
        .expect("Failed to contact database during election id counter init");

    rocket::build()
        .mount("/", api::routes())
        .attach(Shield::default().disable::<NoSniff>())
        .attach(AdHoc::config::<Config>())
        .manage(client)
        .manage(db)
        .manage(notifier)
        .manage(Arc::new(Mutex::new(election_finalizers)))
}

/// Convenient synonym for accessing state.
pub type ElectionFinalizers = Arc<Mutex<RawElectionFinalizers>>;

#[derive(Deserialize)]
pub struct Config {
    hostname: String,
    otp_ttl: u32,
    jwt_secret: String,
    hmac_secret: String,
    recaptcha_secret: String,
    auth_ttl: u32,
}

impl Config {
    /// The hostname the site is running on.
    /// Configured via `HOSTNAME`.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

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

    /// Key used to sign HMACs
    /// Configured via `HMAC_SECRET`.
    pub fn hmac_secret(&self) -> &[u8] {
        self.hmac_secret.as_bytes()
    }

    /// Secret key for reCAPTCHA verification.
    /// Configured via `RECAPTCHA_SECRET`.
    pub fn recaptcha_secret(&self) -> &str {
        &self.recaptcha_secret
    }

    /// Seconds until the authentication token expires
    /// Configured via `AUTH_TTL`.
    pub fn auth_ttl(&self) -> Duration {
        Duration::seconds(self.auth_ttl.into())
    }
}
