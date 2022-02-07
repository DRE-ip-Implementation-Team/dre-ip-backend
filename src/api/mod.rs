use rocket::Route;

mod admin;
mod auth;
mod common;
mod group;
mod public;
mod voting;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(admin::routes());
    routes.extend(public::routes());
    routes.extend(auth::routes());
    routes.extend(group::routes());
    routes.extend(voting::routes());
    routes
}
