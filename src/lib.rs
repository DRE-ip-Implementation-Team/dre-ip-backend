#[macro_use]
extern crate rocket;

#[cfg(test)]
#[macro_use]
extern crate backend_test;

use rocket::{
    shield::{NoSniff, Shield},
    Build, Rocket,
};

pub mod api;
pub mod config;
pub mod error;
pub mod logging;
pub mod model;
pub mod scheduled_task;

pub fn build() -> Rocket<Build> {
    rocket::build()
        .mount("/", api::routes())
        .attach(Shield::default().disable::<NoSniff>())
        .attach(logging::LoggerFairing)
        .attach(config::ConfigFairing) // Must come before most other fairings.
        .attach(config::DatabaseFairing)
        .attach(config::AwsFairing)
        .attach(model::db::election::ElectionFinalizerFairing)
}
