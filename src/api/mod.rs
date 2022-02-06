use rocket::Route;

mod admin;
mod auth;
mod group;
mod public;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(admin::routes());
    routes.extend(public::routes());
    routes.extend(auth::routes());
    routes.extend(group::routes());
    routes
}
