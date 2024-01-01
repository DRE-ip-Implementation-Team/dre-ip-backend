use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::provider::SharedCredentialsProvider;
use aws_sdk_sns::{
    config::{Credentials, Region},
    Client as SnsClient,
};
use chrono::Duration;
use mongodb::Client as MongoClient;
use rocket::futures::TryFutureExt;
use rocket::{
    fairing::{Fairing, Info, Kind},
    Build, Rocket,
};
use serde::Deserialize;

use crate::model::{
    db::admin::ensure_admin_exists,
    mongodb::{ensure_election_id_counter_exists, ensure_indexes_exist, Coll},
};

/// Application configuration, derived from `Rocket.toml` and `ROCKET_*`
/// environment variables. This struct becomes managed state and can be
/// inspected by any endpoint.
#[derive(Deserialize)]
pub struct Config {
    // non-secrets
    hostname: String,
    otp_ttl: u32,
    auth_ttl: u32,
    // secrets
    jwt_secret: String,
    recaptcha_secret: String,
    hmac_secret: String,
}

impl Config {
    /// The hostname the site is running on.
    /// Used in the reCAPTCHA verification API.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Valid lifetime of OTP in seconds.
    pub fn otp_ttl(&self) -> Duration {
        Duration::seconds(self.otp_ttl.into())
    }

    /// Valid lifetime of auth token cookies in seconds.
    pub fn auth_ttl(&self) -> Duration {
        Duration::seconds(self.auth_ttl.into())
    }

    /// Secret key used to encrypt JWTs.
    pub fn jwt_secret(&self) -> &[u8] {
        self.jwt_secret.as_bytes()
    }

    /// Secret key for reCAPTCHA verification.
    pub fn recaptcha_secret(&self) -> &str {
        &self.recaptcha_secret
    }

    /// Secret key used to sign HMACs.
    pub fn hmac_secret(&self) -> &[u8] {
        self.hmac_secret.as_bytes()
    }
}

/// A fairing that loads the application config and puts it in managed state.
/// This could easily be achieved using `AdHoc::config`, but is written out
/// explicitly for symmetry with the other fairings and control over error
/// messages.
pub struct ConfigFairing;

#[rocket::async_trait]
impl Fairing for ConfigFairing {
    fn info(&self) -> Info {
        Info {
            name: "Config",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        // Load the config.
        let config = match rocket.figment().extract::<Config>() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load application config");
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        // Manage the state.
        rocket = rocket.manage(config);
        Ok(rocket)
    }
}

/// Configuration for the database.
#[derive(Deserialize)]
struct DbConfig {
    // secrets
    db_uri: String,
}

/// A fairing that loads the MongoDB config, connects to the database,
/// performs any setup necessary, and places both a `Client` and a `Database`
/// into managed state.
pub struct DatabaseFairing;

#[rocket::async_trait]
impl Fairing for DatabaseFairing {
    fn info(&self) -> Info {
        Info {
            name: "MongoDB",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        // Load the config.
        let config = match rocket.figment().extract::<DbConfig>() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load database config");
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };
        info!("Loaded database config, connecting...");
        // Construct the connection.
        let client = match MongoClient::with_uri_str(config.db_uri).await {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to connect to database: {e}");
                return Err(rocket);
            }
        };
        let db = client.database(&get_database_name());

        // Ensure the required indexes exist.
        if let Err(e) = ensure_indexes_exist(&db).await {
            error!("Failed to connect to database: {e}");
            return Err(rocket);
        }

        // Ensure there is at least one admin user and the global election ID counter exists.
        let admins = Coll::from_db(&db);
        let counters = Coll::from_db(&db);
        if let Err(e) = ensure_admin_exists(&admins)
            .and_then(|_| ensure_election_id_counter_exists(&counters))
            .await
        {
            error!("Failed to connect to database: {e}");
            return Err(rocket);
        }
        info!("...database connection online!");

        // Manage the state.
        rocket = rocket.manage(client).manage(db);
        Ok(rocket)
    }
}

/// Get the name of the database to use (production version).
#[cfg(not(test))]
fn get_database_name() -> String {
    "dreip".to_string()
}

/// Get the name of the database to use (test version).
/// Use a random name to avoid collisions between tests.
#[cfg(test)]
fn get_database_name() -> String {
    let random: u32 = rand::random();
    let db = format!("test{random}");
    info!("Using database {db}");
    db
}

/// Configuration for the AWS connection.
#[derive(Deserialize)]
struct AwsConfig {
    // non-secrets
    aws_region: String,
    aws_access_key_id: String,
    // secrets
    aws_secret_access_key: String,
}

/// A fairing that loads the AWS config and places an SNS `Client` into
/// managed state.
pub struct AwsFairing;

#[rocket::async_trait]
impl Fairing for AwsFairing {
    fn info(&self) -> Info {
        Info {
            name: "AWS SNS",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        // Load the config.
        let config = match rocket.figment().extract::<AwsConfig>() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load AWS config");
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };
        // Construct the connection.
        let aws_config = SdkConfig::builder()
            .region(Region::new(config.aws_region))
            .credentials_provider(SharedCredentialsProvider::new(Credentials::new(
                config.aws_access_key_id,
                config.aws_secret_access_key,
                None,
                None,
                "rocket config",
            )))
            .behavior_version(BehaviorVersion::latest())
            .build();
        let client = SnsClient::new(&aws_config);
        info!("Loaded Amazon SNS config");

        // Manage the state.
        rocket = rocket.manage(client);
        Ok(rocket)
    }
}
