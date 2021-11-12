#[macro_use]
extern crate rocket;
use crate::model::{Otp, User};
use mongodb::Client;
use rocket::{Build, Rocket};
use serde::Deserialize;

mod api;
pub mod model;

pub async fn build() -> Rocket<Build> {
    let rocket = rocket::build();
    let figment = rocket.figment();

    let client = Client::with_uri_str("mongodb://localhost:27017")
        .await
        .expect("Could not connect to database");
    let db = client.database("dreip");

    let users = db.collection::<User>("users");
    let otps = db.collection::<Otp>("otps");

    let admin_password = figment
        .extract_inner("admin_password")
        .unwrap_or(AdminPassword(String::new()));

    rocket
        .mount("/", api::routes())
        .manage(users)
        .manage(otps)
        .manage(admin_password)
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
