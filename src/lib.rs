#[macro_use]
extern crate rocket;
use crate::model::{otp::Otp, user::User};
use mongodb::Client;
use once_cell::sync::OnceCell;
use rocket::{Build, Rocket};
use serde::Deserialize;

pub mod api;
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

    let users = db.collection::<User>("users");
    let otps = db.collection::<Otp>("otps");

    let admin_password: AdminPassword = figment
        .extract_inner("admin_password")
        .unwrap_or(AdminPassword(String::new()));

    CONFIG.set(figment.extract::<Config>().unwrap_or_default());

    rocket
        .mount("/", api::routes())
        .manage(users)
        .manage(otps)
        .manage(admin_password)
}

/// Contains the configuration information for the program.
///
/// Fields can be extracted from `Config` using the [`conf`] macro.
pub static CONFIG: OnceCell<Config> = OnceCell::new();

#[derive(Deserialize)]
pub struct Config {
    otp_ttl: u64,
    jwt_secret: &'static [u8],
    jwt_duration: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            otp_ttl: 60 * 5, // 5 mins
            jwt_secret: "secret".as_bytes(),
            jwt_duration: 60 * 60 * 2, // 2 hours
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

/// Compare-only administrator password.
///
/// The inner [`String`] is deliberately unexposed to prevent accidental mutation or output.
///
/// The value is sourced at the beginning of execution under the name `admin_password`.
///
/// An [`AdminPassword`] can only be compared with [`str`] values.
#[derive(Deserialize)]
struct AdminPassword(String);

impl PartialEq<str> for AdminPassword {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<AdminPassword> for str {
    fn eq(&self, other: &AdminPassword) -> bool {
        self == other.0
    }
}
