use rocket::Route;

mod auth;
mod election_management;
mod information;
mod voting;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(information::routes());
    routes.extend(auth::routes());
    routes.extend(election_management::routes());
    routes.extend(voting::routes());
    routes
}
