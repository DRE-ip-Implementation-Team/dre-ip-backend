#[macro_use] extern crate rocket;
use rocket::{Build, Rocket};

mod api;

pub fn build() -> Rocket<Build> {
    rocket::build().mount("/", api::routes())
}
