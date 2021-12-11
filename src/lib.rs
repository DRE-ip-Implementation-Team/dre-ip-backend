#[macro_use]
extern crate rocket;

use crate::model::{
    election::{Ballot, Election},
    voter::db::DbVoter,
};

use chrono::Duration;
use model::{
    admin::{db::DbAdmin, Admin},
    voter::Voter,
};
use mongodb::Client;
use once_cell::sync::OnceCell;
use rocket::{Build, Rocket};
use serde::Deserialize;

pub mod api;
pub mod error;
pub mod model;

pub async fn build() -> Rocket<Build> {
    let rocket = rocket::build();
    let figment = rocket.figment();

    let db_host = figment.extract_inner("db_host").unwrap_or("localhost");
    let db_port = figment.extract_inner("db_port").unwrap_or(27017);
    let client = Client::with_uri_str(format!("mongodb://{}:{}", db_host, db_port))
        .await
        .expect("Could not connect to database");
    let db = client.database("dreip");

    let get_admins = db.collection::<DbAdmin>("admin");
    let put_admins = db.collection::<Admin>("admin");
    let get_users = db.collection::<DbVoter>("voters");
    let put_users = db.collection::<Voter>("voters");
    let elections = db.collection::<Election>("elections");
    let ballots = db.collection::<Ballot>("ballots");

    CONFIG.set(figment.extract::<Config>().unwrap_or_default());

    rocket
        .mount("/", api::routes())
        .manage(get_admins)
        .manage(put_admins)
        .manage(get_users)
        .manage(put_users)
        .manage(elections)
        .manage(ballots)
}

/// Contains the configuration information for the program.
///
/// Fields can be extracted from `Config` using the [`conf`] macro.
pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize)]
pub struct Config {
    otp_ttl: u64,
    jwt_secret: &'static [u8],
    auth_ttl: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            otp_ttl: Duration::minutes(5).num_seconds() as u64,
            jwt_secret: b"$!~B.4uQLt@d*K5w",
            auth_ttl: Duration::days(1).num_seconds() as u64,
        }
    }
}

/// Extracts a `var` from the configuration environment
///
/// # Usage
///
/// `CONFIG` must be a [`OnceCell`] declared at the crate root and must wrap a named struct containing a field with the identifier `var`.
///
/// # Safety
///
/// `CONFIG` must be initialized once at program start and modified nowhere else.
#[macro_export]
macro_rules! conf {
    ($var:ident) => {
        // SAFETY: `CONFIG` is initialized once at program start and modified nowhere else
        unsafe { $crate::CONFIG.get_unchecked() }.$var
    };
}
